# -*- coding: utf-8 -*-
# @Time    : 2026/11/2 00:00
# @Author  : fzf
# @FileName: test_packaging.py
# @Software: resource-pulse

import configparser
import struct
import subprocess
import tempfile
import unittest
from unittest.mock import patch
import xml.etree.ElementTree as ET

from pathlib import Path
from build_deb import APP_ICON_SIZES, TRAY_ICON_SIZES, APP_ID, ROOT, dependencies, install_hooks, install_icons, third_party_notices
from verify_runtime import bus, registered_menu, unwrap, validate_tray_icons


class PackagingTests(unittest.TestCase):
    def test_third_party_license_texts_keep_notices_without_local_paths(self):
        with tempfile.TemporaryDirectory(prefix="pulse-license-test-") as directory:
            root = Path(directory)
            (root / "LICENSE").write_text("Copyright example authors\nPermission is granted.\n")
            package = {
                "name": "example", "version": "1.2.3", "source": "registry+https://github.com/rust-lang/crates.io-index",
                "license": "MIT", "manifest_path": str(root / "Cargo.toml"),
            }
            notice = third_party_notices({"packages": [package]})
            self.assertIn("Copyright example authors", notice)
            self.assertIn("https://crates.io/api/v1/crates/example/1.2.3/download", notice)
            self.assertNotIn(directory, notice)

    def test_launch_identity_and_dock_icon_match(self):
        entry = configparser.ConfigParser(interpolation=None)
        entry.read(ROOT / f"packaging/{APP_ID}.desktop", encoding="utf-8")
        app = entry["Desktop Entry"]
        self.assertEqual(app["Exec"], "resource-pulse")
        self.assertEqual(app["Icon"], f"/usr/share/icons/hicolor/scalable/apps/{APP_ID}.svg")
        self.assertEqual(app["StartupWMClass"], APP_ID)
        self.assertEqual(app["Terminal"], "false")

    def test_dependencies_include_linked_and_dynamically_loaded_libraries(self):
        result = dependencies("shlibs:Depends=libc6 (>= 2.34), libvulkan1\n")
        self.assertIn("libc6 (>= 2.34)", result)
        self.assertIn("fonts-noto-cjk", result)
        self.assertIn("librsvg2-common", result)
        self.assertIn("gtk-update-icon-cache", result)
        self.assertEqual(result.count("libvulkan1"), 1)
        with self.assertRaises(ValueError):
            dependencies("")

    def test_mascot_sources_and_embedded_png_sizes(self):
        for name in ["pulse.svg", "pulse-tray.svg", "pulse-symbolic.svg"]:
            self.assertEqual(ET.parse(ROOT / "assets" / name).getroot().tag, "{http://www.w3.org/2000/svg}svg")
        for size in [32, 256]:
            data = (ROOT / f"assets/pulse-{size}.png").read_bytes()
            self.assertEqual(data[:8], b"\x89PNG\r\n\x1a\n")
            self.assertEqual(struct.unpack("!II", data[16:24]), (size, size))

    def test_deb_icons_are_native_size_at_both_desktop_scales(self):
        with tempfile.TemporaryDirectory(prefix="pulse-icon-package-") as directory:
            root = Path(directory)
            install_icons(root)
            for size in APP_ICON_SIZES:
                for scale in [1, 2]:
                    folder = f"{size}x{size}" + ("@2" if scale == 2 else "")
                    icon = root / f"usr/share/icons/hicolor/{folder}/apps/{APP_ID}.png"
                    self.assertEqual(struct.unpack("!II", icon.read_bytes()[16:24]), (size * scale,) * 2)
            for size in TRAY_ICON_SIZES:
                icon = root / f"usr/share/icons/hicolor/{size}x{size}/status/{APP_ID}-tray.png"
                self.assertEqual(struct.unpack("!II", icon.read_bytes()[16:24]), (size,) * 2)
            self.assertTrue((root / f"usr/share/icons/hicolor/scalable/apps/{APP_ID}.svg").is_file())
            self.assertTrue((root / f"usr/share/icons/hicolor/scalable/status/{APP_ID}-tray.svg").is_file())

    def test_icon_cache_hooks_are_executable_valid_shell_and_do_not_modify_dock(self):
        with tempfile.TemporaryDirectory(prefix="pulse-icon-hooks-") as directory:
            root = Path(directory)
            install_hooks(root)
            for name in ["postinst", "postrm"]:
                script = root / name
                self.assertEqual(script.stat().st_mode & 0o777, 0o755)
                subprocess.run(["sh", "-n", str(script)], check=True)
                source = script.read_text()
                self.assertIn("gtk-update-icon-cache", source)
                self.assertNotIn("gsettings", source)
                self.assertNotIn("plank", source)

    def test_dbus_values_handle_scalar_and_variant_wrapping(self):
        self.assertEqual(unwrap({"type": "u", "data": 42}), 42)
        self.assertEqual(unwrap({"type": "s", "data": ["暂停监控"]}), "暂停监控")
        self.assertEqual(unwrap({"type": "v", "data": {"type": "s", "data": "Pulse"}}), "Pulse")

    def test_tray_payload_rejects_missing_names_bad_sizes_and_invisible_pixels(self):
        pixmaps = [[size, size, [255, 12, 34, 56] * size * size] for size in TRAY_ICON_SIZES]
        self.assertEqual(validate_tray_icons("", pixmaps)["native_pixel_sizes"], TRAY_ICON_SIZES)
        with self.assertRaises(RuntimeError):
            validate_tray_icons(APP_ID, pixmaps)
        with self.assertRaises(RuntimeError):
            validate_tray_icons("", pixmaps[:1])
        with self.assertRaises(RuntimeError):
            validate_tray_icons("", [[32, 32, [0] * 32 * 32 * 4]])

    def test_global_menu_registration_requires_a_real_menu_endpoint(self):
        props = '_KDE_NET_WM_APPMENU_SERVICE_NAME(STRING) = ":1.42"\n_KDE_NET_WM_APPMENU_OBJECT_PATH(STRING) = "/io/github/fzf/Pulse/MenuBar"\n'
        with patch("verify_runtime.run", return_value=props), patch("verify_runtime.menu_items", return_value={"Pulse": 1}):
            self.assertEqual(registered_menu(42), (":1.42", "/io/github/fzf/Pulse/MenuBar"))
        with patch("verify_runtime.run", return_value="_KDE_NET_WM_APPMENU_OBJECT_PATH: not found.\n"):
            self.assertIsNone(registered_menu(42))

    def test_negative_menu_depth_is_not_parsed_as_a_busctl_option(self):
        with patch("verify_runtime.run", return_value='{"type":"u","data":4}') as command:
            self.assertEqual(bus("call", "destination", "/MenuBar", "interface", "GetLayout", "iias", "0", "-1", "0")["data"], 4)
            self.assertEqual(command.call_args[0][4], "--")


if __name__ == "__main__":
    unittest.main()
