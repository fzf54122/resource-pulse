# -*- coding: utf-8 -*-
# @Time    : 2026/11/2 00:00
# @Author  : fzf
# @FileName: build_deb.py
# @Software: resource-pulse

"""Build a local Debian package. Never installs packages or changes desktop preferences."""

import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tempfile


ROOT = Path(__file__).resolve().parent.parent
APP_ID = "io.github.fzf.Pulse"
APP_ICON_SIZES = [16, 22, 24, 32, 48, 64, 96, 128, 256, 512]
TRAY_ICON_SIZES = [16, 22, 24, 32, 48, 64, 96]
RUNTIME_DEPENDENCIES = [
    "fonts-noto-cjk",
    "hicolor-icon-theme",
    "gtk-update-icon-cache",
    "librsvg2-common",
    "libvulkan1",
    "libfontconfig1",
    "libfreetype6",
    "libxkbcommon0",
    "libxkbcommon-x11-0",
    "libwayland-client0",
    "libwayland-cursor0",
    "libx11-xcb1",
    "libxcb-shape0",
    "libxcb-xfixes0",
]


def run(*args, cwd=ROOT):
    return subprocess.run(args, cwd=cwd, check=True, text=True, capture_output=True).stdout.strip()


def dependencies(shlibs):
    lines = [line for line in shlibs.splitlines() if line.startswith("shlibs:Depends=")]
    if len(lines) != 1:
        raise ValueError("dpkg-shlibdeps did not produce a dependency list")
    required = lines[0].split("=", 1)[1].split(", ")
    names = {value.split(" ", 1)[0] for value in required}
    required.extend(value for value in RUNTIME_DEPENDENCIES if value not in names)
    return ", ".join(required)


def copy(source, destination):
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(source, destination)
    destination.chmod(0o644)

def install_icons(stage):
    theme = stage / "usr/share/icons/hicolor"
    copy(ROOT / "assets/pulse.svg", theme / f"scalable/apps/{APP_ID}.svg")
    copy(ROOT / "assets/pulse-tray.svg", theme / f"scalable/status/{APP_ID}-tray.svg")
    copy(ROOT / "assets/pulse-symbolic.svg", theme / f"symbolic/status/{APP_ID}-symbolic.svg")
    for size in APP_ICON_SIZES:
        for scale in [1, 2]:
            directory = f"{size}x{size}" + ("@2" if scale == 2 else "")
            icon = theme / f"{directory}/apps/{APP_ID}.png"
            icon.parent.mkdir(parents=True, exist_ok=True)
            pixels = str(size * scale)
            run("rsvg-convert", "-w", pixels, "-h", pixels, str(ROOT / "assets/pulse.svg"), "-o", str(icon))
            icon.chmod(0o644)
    for size in TRAY_ICON_SIZES:
        copy(ROOT / f"assets/tray/pulse-{size}.png", theme / f"{size}x{size}/status/{APP_ID}-tray.png")

def install_hooks(debian):
    for name in ["postinst", "postrm"]:
        copy(ROOT / "packaging" / name, debian / name)
        (debian / name).chmod(0o755)

def third_party_notices(metadata):
    sections = [
        "Third-party dependency inventory and license texts",
        "Generated from Cargo.lock and locally resolved source packages.",
        "This conservative inventory may include build, test and other-platform dependencies.",
        "Upstream source packages are unmodified. Their license terms remain applicable.",
    ]
    texts = {}
    for package in sorted(metadata["packages"], key=lambda p: (p["name"], p["version"])):
        if not package.get("source"):
            continue
        name, version = package["name"], package["version"]
        root = Path(package["manifest_path"]).parent
        sections.extend([
            "", f"## {name} {version}",
            f"Declared license: {package.get('license') or 'See license files'}",
            f"Source: {package['source']}",
        ])
        if package["source"].startswith("registry+"):
            sections.append(f"Source archive: https://crates.io/api/v1/crates/{name}/{version}/download")
        elif package.get("repository"):
            sections.append(f"Repository: {package['repository']}")
        files = set()
        directories = [root]
        if package["source"].startswith("git+"):
            directories.extend(list(root.parents)[:3])
        for directory in directories:
            for file in directory.iterdir():
                if file.is_file() and file.name.upper().startswith(("LICENSE", "COPYING", "NOTICE", "UNLICENSE")):
                    files.add(file)
            for subdir in ["licenses", "LICENSES"]:
                if (directory / subdir).is_dir():
                    files.update(file for file in (directory / subdir).rglob("*") if file.is_file())
        if package.get("license_file"):
            files.add(root / package["license_file"])
        for file in sorted(files):
            content = file.read_text(encoding="utf-8", errors="replace")
            digest = hashlib.sha256(content.encode()).hexdigest()
            texts[digest] = content
            sections.append(f"License/notice: {file.name} (text {digest})")
        if not files:
            sections.append("License text: see the exact upstream source archive or repository above.")
    for digest, content in texts.items():
        sections.extend(["", f"## License/notice text {digest}", content])
    return "\n".join(sections) + "\n"


def build(skip_build, output):
    for tool in ["cargo", "dpkg", "dpkg-deb", "dpkg-shlibdeps", "rsvg-convert", "desktop-file-validate"]:
        if shutil.which(tool) is None:
            raise RuntimeError(f"Missing build tool: {tool}")
    if not skip_build:
        subprocess.run(["cargo", "build", "--release", "--locked"], cwd=ROOT, check=True)
    binary = ROOT / "target/release/resource-pulse"
    if not binary.is_file():
        raise RuntimeError("Release binary missing; run cargo build --release --locked first")
    metadata = json.loads(run("cargo", "metadata", "--format-version", "1", "--locked", "--offline"))
    version = next(p["version"] for p in metadata["packages"] if p["name"] == "resource-pulse")
    architecture = run("dpkg", "--print-architecture")
    output.mkdir(parents=True, exist_ok=True)
    artifact = output / f"resource-pulse_{version}_{architecture}.deb"
    with tempfile.TemporaryDirectory(prefix="pulse-deb-", dir=ROOT / "target") as temporary:
        work = Path(temporary)
        stage = work / "root"
        debian = stage / "DEBIAN"
        debian.mkdir(parents=True)
        copy(binary, stage / "usr/bin/resource-pulse")
        (stage / "usr/bin/resource-pulse").chmod(0o755)
        desktop = stage / f"usr/share/applications/{APP_ID}.desktop"
        copy(ROOT / f"packaging/{APP_ID}.desktop", desktop)
        run("desktop-file-validate", str(desktop))
        install_icons(stage)
        install_hooks(debian)
        copy(ROOT / "README.md", stage / "usr/share/doc/resource-pulse/README.md")
        for name in ["LICENSE", "THIRD_PARTY_NOTICES.md"]:
            copy(ROOT / name, stage / "usr/share/doc/resource-pulse" / name)
        (stage / "usr/share/doc/resource-pulse/THIRD_PARTY_LICENSES.txt").write_text(
            third_party_notices(metadata), encoding="utf-8",
        )
        control_base = (
            "Section: utils\nPriority: optional\n"
            "Maintainer: fzf <noreply@localhost>\n"
            "Description: Pulse local system resource monitor\n"
            " Native resource monitoring with process details and a system tray.\n"
        )
        (work / "debian").mkdir()
        (work / "debian/control").write_text(
            f"Source: resource-pulse\n{control_base}\nPackage: resource-pulse\nArchitecture: any\n{control_base}",
            encoding="utf-8",
        )
        shlibs = run("dpkg-shlibdeps", "-O", "-e" + str(binary), cwd=work)
        installed_size = sum(p.stat().st_size for p in stage.rglob("*") if p.is_file()) // 1024
        (debian / "control").write_text(
            f"Package: resource-pulse\nVersion: {version}\nArchitecture: {architecture}\n"
            f"Installed-Size: {installed_size}\nDepends: {dependencies(shlibs)}\n"
            f"{control_base}",
            encoding="utf-8",
        )
        (debian / "control").chmod(0o644)
        run("dpkg-deb", "--root-owner-group", "--build", str(stage), str(artifact))
    print(artifact)
    return artifact


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--skip-build", action="store_true", help="Package the existing release binary")
    parser.add_argument("--output", type=Path, default=ROOT / "target/debian")
    args = parser.parse_args()
    try:
        build(args.skip_build, args.output.resolve())
    except (RuntimeError, ValueError, subprocess.CalledProcessError) as error:
        detail = getattr(error, "stderr", None) or str(error)
        parser.exit(1, f"Deb build failed: {detail}\n")


if __name__ == "__main__":
    main()
