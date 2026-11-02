# -*- coding: utf-8 -*-
# @Time    : 2026/11/2 00:00
# @Author  : fzf
# @FileName: verify_runtime.py
# @Software: resource-pulse

"""Verify an installed Pulse on an X11 desktop using an isolated preferences directory.

Opens and controls only the process started by this script. Measures wall-clock CPU
against one logical core; results depend on the device, driver and current load.
Requires busctl, xdotool, and a working StatusNotifierItem host.
"""

import argparse
import ctypes
import ctypes.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time


APP_ID = "io.github.fzf.Pulse"
OBJECT_PATH = "/io/github/fzf/Pulse"


def run(*args):
    return subprocess.run(args, check=True, capture_output=True, text=True, timeout=10).stdout.strip()


def bus(*args):
    result = run("busctl", "--user", "--timeout=3s", "--json=short", "--", *args)
    return json.loads(result) if result else None


def body(result):
    return result["data"]


def unwrap(value):
    while isinstance(value, dict) and "type" in value and "data" in value:
        value = value["data"]
    if isinstance(value, list) and len(value) == 1:
        return unwrap(value[0])
    return value


def menu_items(service, path="/MenuBar"):
    result = bus("call", service, path, "com.canonical.dbusmenu", "GetLayout", "iias", "0", "-1", "0")
    entries = {}

    def walk(value):
        if isinstance(value, list):
            if len(value) == 3 and isinstance(value[0], int) and isinstance(value[1], dict):
                label = unwrap(value[1].get("label", ""))
                if isinstance(label, str):
                    entries[label.replace("_", "")] = value[0]
            for child in value:
                walk(child)
        elif isinstance(value, dict):
            for child in value.values():
                walk(child)
    walk(body(result))
    return entries


def click_menu(service, label, path="/MenuBar"):
    entries = menu_items(service, path)
    if label not in entries:
        raise RuntimeError(f"Tray menu entry missing: {label}")
    bus("call", service, path, "com.canonical.dbusmenu", "Event",
        "isvu", str(entries[label]), "clicked", "s", "", "0")

def tray_service(pid):
    items = body(bus("get-property", "org.kde.StatusNotifierWatcher", "/StatusNotifierWatcher",
        "org.kde.StatusNotifierWatcher", "RegisteredStatusNotifierItems"))
    if isinstance(items, str):
        items = [items]
    prefix = f"org.kde.StatusNotifierItem-{pid}-"
    return next((item.split("/", 1)[0] for item in items if item.startswith(prefix)), None)

def registered_menu(window):
    props = run("xprop", "-id", str(window), "_KDE_NET_WM_APPMENU_SERVICE_NAME", "_KDE_NET_WM_APPMENU_OBJECT_PATH")
    values = {}
    for line in props.splitlines():
        if ' = "' in line and line.endswith('"'):
            key, value = line.split(' = "', 1)
            values[key.split("(", 1)[0]] = value[:-1]
    service = values.get("_KDE_NET_WM_APPMENU_SERVICE_NAME")
    path = values.get("_KDE_NET_WM_APPMENU_OBJECT_PATH")
    if not service or not path or path == "/":
        return None
    menu_items(service, path)
    return service, path

def validate_tray_icons(name, pixmaps):
    if name:
        path = Path(name)
        if not path.is_absolute() or not path.is_file():
            raise RuntimeError("Tray IconName must point to a readable image, not an unresolved theme name")
    sizes = []
    for width, height, data in pixmaps:
        if width <= 0 or width != height or len(data) != width * height * 4:
            raise RuntimeError("Invalid tray ARGB pixel buffer")
        if not any(data[index] for index in range(0, len(data), 4)):
            raise RuntimeError("Tray pixel buffer is fully transparent")
        sizes.append(width)
    if sizes != [16, 22, 24, 32, 48, 64, 96]:
        raise RuntimeError("Tray icons are missing native-size or high-DPI representations")
    return {"readable_icon_path": bool(name), "native_pixel_sizes": sizes}

def verify_tray_icons(service):
    name = body(bus("get-property", service, "/StatusNotifierItem", "org.kde.StatusNotifierItem", "IconName"))
    pixmaps = body(bus("get-property", service, "/StatusNotifierItem", "org.kde.StatusNotifierItem", "IconPixmap"))
    return validate_tray_icons(unwrap(name), pixmaps)


def wait_for(predicate, description, timeout=8):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        try:
            result = predicate()
            if result:
                return result
        except subprocess.CalledProcessError:
            pass
        time.sleep(0.1)
    raise RuntimeError(f"Timed out: {description}")


def process_metrics(pid):
    path = Path("/proc") / str(pid)
    values = (path / "stat").read_text().rsplit(")", 1)[1].split()
    status = (path / "status").read_text().splitlines()
    rss = int(next(value.split()[1] for value in status if value.startswith("VmRSS:")))
    return int(values[11]) + int(values[12]), rss


def measure(pid, seconds):
    before = process_metrics(pid)
    started = time.monotonic()
    time.sleep(seconds)
    after = process_metrics(pid)
    elapsed = time.monotonic() - started
    return {
        "seconds": round(elapsed, 2),
        "cpu_percent_one_core": round((after[0] - before[0]) / os.sysconf("SC_CLK_TCK") / elapsed * 100, 2),
        "rss_mib": round(after[1] / 1024, 1),
    }


def request_close(window):
    class Data(ctypes.Union):
        _fields_ = [("longs", ctypes.c_long * 5)]

    class Message(ctypes.Structure):
        _fields_ = [
            ("type", ctypes.c_int), ("serial", ctypes.c_ulong), ("send_event", ctypes.c_int),
            ("display", ctypes.c_void_p), ("window", ctypes.c_ulong),
            ("message_type", ctypes.c_ulong), ("format", ctypes.c_int), ("data", Data),
        ]

    class Event(ctypes.Union):
        _fields_ = [("message", Message), ("padding", ctypes.c_long * 24)]

    x11 = ctypes.CDLL(ctypes.util.find_library("X11"))
    x11.XOpenDisplay.argtypes = [ctypes.c_char_p]
    x11.XOpenDisplay.restype = ctypes.c_void_p
    display = x11.XOpenDisplay(None)
    if not display:
        raise RuntimeError("Cannot open X11 display")
    try:
        x11.XInternAtom.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_int]
        x11.XInternAtom.restype = ctypes.c_ulong
        event = Event()
        event.message.type = 33
        event.message.send_event = 1
        event.message.display = display
        event.message.window = int(window)
        event.message.message_type = x11.XInternAtom(display, b"WM_PROTOCOLS", 0)
        event.message.format = 32
        event.message.data.longs[0] = x11.XInternAtom(display, b"WM_DELETE_WINDOW", 0)
        x11.XSendEvent.argtypes = [ctypes.c_void_p, ctypes.c_ulong, ctypes.c_int, ctypes.c_long, ctypes.POINTER(Event)]
        if not x11.XSendEvent(display, int(window), 0, 0, ctypes.byref(event)):
            raise RuntimeError("Cannot send application close request")
        x11.XFlush.argtypes = [ctypes.c_void_p]
        x11.XFlush(display)
    finally:
        x11.XCloseDisplay.argtypes = [ctypes.c_void_p]
        x11.XCloseDisplay(display)


def verify(binary, seconds, check_package=True, require_global_menu=False):
    if check_package:
        if run("dpkg-query", "-W", "-f=${db:Status-Status}", "resource-pulse") != "installed":
            raise RuntimeError("resource-pulse is not installed")
        if run("dpkg", "--verify", "resource-pulse"):
            raise RuntimeError("Installed package files differ from package metadata")
        if not Path(f"/usr/share/applications/{APP_ID}.desktop").is_file():
            raise RuntimeError("Installed application entry missing")
        for size in [16, 24, 32, 48, 64, 128, 256, 512]:
            if not Path(f"/usr/share/icons/hicolor/{size}x{size}/apps/{APP_ID}.png").is_file():
                raise RuntimeError(f"Installed {size}px icon missing")
    with tempfile.TemporaryDirectory(prefix="pulse-runtime-") as temporary:
        env = dict(os.environ, XDG_CONFIG_HOME=temporary, XDG_CACHE_HOME=str(Path(temporary) / "cache"))
        with tempfile.TemporaryFile() as log:
            process = subprocess.Popen([str(binary)], env=env, stdout=log, stderr=log)
            service = None
            try:
                time.sleep(3)
                if process.poll() is not None:
                    raise RuntimeError("Test process exited early. Close any existing Pulse instance and retry.")
                owner = unwrap(body(bus("call", "org.freedesktop.DBus", "/org/freedesktop/DBus",
                    "org.freedesktop.DBus", "GetConnectionUnixProcessID", "s", APP_ID)))
                if owner != process.pid:
                    raise RuntimeError("Refusing to control a Pulse instance not started by this test")
                service = wait_for(lambda: tray_service(process.pid), "tray host registration")
                wait_for(lambda: menu_items(service).get("打开 Pulse"), "tray registration")
                tray_icons = verify_tray_icons(service)
                windows = run("xdotool", "search", "--class", APP_ID).splitlines()
                if len(windows) != 1:
                    raise RuntimeError(f"Expected one application window, found {len(windows)}")
                window = windows[0]
                global_menu = None
                if require_global_menu:
                    global_menu = wait_for(lambda: registered_menu(window), "dde-top-panel menu registration")
                    labels = menu_items(*global_menu)
                    if not all(label in labels for label in ["Pulse", "查看", "监控", "窗口"]):
                        raise RuntimeError("Global menu is missing application menu groups")
                second = subprocess.run([str(binary)], env=env, capture_output=True, timeout=8)
                if second.returncode != 0:
                    raise RuntimeError("Second launch did not activate the existing instance")
                if len(run("xdotool", "search", "--class", APP_ID).splitlines()) != 1:
                    raise RuntimeError("Second launch created another window")
                report = {
                    "installed_files_verified": check_package,
                    "tray_menu": True,
                    "tray_icons": tray_icons,
                    "single_instance": True,
                    "window_icon_identity": APP_ID,
                    "global_menu_registered": global_menu is not None,
                    "foreground": measure(process.pid, seconds),
                }
                if global_menu:
                    click_menu(global_menu[0], "暂停监控", global_menu[1])
                else:
                    click_menu(service, "暂停监控")
                wait_for(lambda: "继续监控" in menu_items(service), "pause state synchronization")
                report["paused"] = measure(process.pid, seconds)
                click_menu(service, "继续监控")
                wait_for(lambda: "暂停监控" in menu_items(service), "resume state synchronization")
                if global_menu:
                    wait_for(lambda: "暂停监控" in menu_items(*global_menu), "global menu state synchronization")
                request_close(window)
                time.sleep(0.5)
                if process.poll() is not None:
                    raise RuntimeError("Closing the window unexpectedly terminated a tray-enabled application")
                report["close_to_tray"] = True
                report["background"] = measure(process.pid, seconds)
                bus("call", service, "/StatusNotifierItem", "org.kde.StatusNotifierItem", "Activate", "ii", "0", "0")
                click_menu(service, "退出 Pulse")
                if process.wait(timeout=8) != 0:
                    raise RuntimeError("Tray quit did not shut down cleanly")
                report["tray_quit"] = True
                report["preference_isolation"] = True
                return report
            finally:
                if process.poll() is None:
                    process.terminate()
                    process.wait(timeout=8)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=Path("/usr/bin/resource-pulse"))
    parser.add_argument("--seconds", type=float, default=10)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--skip-package-check", action="store_true")
    parser.add_argument("--require-global-menu", action="store_true")
    args = parser.parse_args()
    if args.seconds < 1 or args.seconds > 300:
        parser.error("--seconds must be between 1 and 300")
    try:
        report = verify(args.binary.resolve(), args.seconds, not args.skip_package_check, args.require_global_menu)
    except (RuntimeError, OSError, subprocess.SubprocessError) as error:
        parser.exit(1, f"Runtime verification failed: {error}\n")
    result = json.dumps(report, ensure_ascii=False, indent=2)
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(result + "\n", encoding="utf-8")
    print(result)


if __name__ == "__main__":
    main()
