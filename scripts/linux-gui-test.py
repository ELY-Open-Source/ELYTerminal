#!/usr/bin/env python3
"""Checks an X11 window and PTY, leaving user settings alone."""

import json
import os
from pathlib import Path
import re
import shlex
import shutil
import signal
import struct
import subprocess
import sys
import tempfile
import time

from PIL import Image, ImageStat


def output(args):
    return subprocess.check_output(args, text=True, stderr=subprocess.DEVNULL, timeout=5)


def app_window(process_id):
    tree = output(["xwininfo", "-root", "-tree"])
    for window_id in re.findall(r"^\s+(0x[0-9a-f]+) ", tree, re.M):
        prop = output(["xprop", "-id", window_id, "_NET_WM_PID"])
        found = re.search(r"= (\d+)", prop)
        if not found:
            continue
        candidate = int(found.group(1))
        for _ in range(16):
            if candidate == process_id:
                if "IsViewable" in output(["xwininfo", "-id", window_id]):
                    return window_id
                break
            try:
                status = Path("/proc", str(candidate), "status").read_text()
                candidate = int(re.search(r"^PPid:\s+(\d+)", status, re.M).group(1))
            except (FileNotFoundError, AttributeError):
                break
    return None


def painted(window_id, base):
    dump = base / "window.xwd"
    subprocess.run(["xwd", "-silent", "-screen", "-id", window_id, "-out", str(dump)],
                   check=True, timeout=5)
    raw = dump.read_bytes()
    header = struct.unpack(">25I", raw[:100])
    if header[11] != 32:
        raise RuntimeError("linux-gui-test: expected a 32-bit X11 capture")
    frame = Image.frombytes(
        "RGB", (header[4], header[5]), raw[header[0] + header[19] * 12 :],
        "raw", "BGRX" if header[7] == 0 else "XRGB", header[12], 1,
    )
    frame.save(base / "window.png")
    return sum(ImageStat.Stat(frame).stddev) > 10


def main():
    if len(sys.argv) < 2 or os.getuid() == 0:
        raise RuntimeError("linux-gui-test: pass a program and run as a regular user")
    if not os.environ.get("DISPLAY"):
        raise RuntimeError("linux-gui-test: requires X11; use xvfb-run for CI")
    for tool in ("xwininfo", "xprop", "xwd"):
        if not shutil.which(tool):
            raise RuntimeError(f"linux-gui-test: missing {tool}")
    parent = os.environ.get("LINUX_TEST_OUTPUT")
    if parent:
        Path(parent).mkdir(parents=True, exist_ok=True)
    base = Path(tempfile.mkdtemp(prefix="elyterminal-gui-", dir=parent))
    shell = base / "smoke-shell.sh"
    proof = base / "shell-environment.txt"
    shell.write_text(
        '#!/bin/sh\nprintf "PACKAGING_GUI_OK\\nChinese fonts: 终端测试\\n"\n'
        'printf "%s\\n" "${LD_LIBRARY_PATH-unset}" > ' + shlex.quote(str(proof))
        + "\nsleep 25\n"
    )
    shell.chmod(0o755)
    config = base / "config" / "ELYTerminal"
    config.mkdir(parents=True)
    (config / "settings.json").write_text(json.dumps({
        "shell": str(shell), "check_updates": False, "ely": False,
        "show_panel": False, "background_opacity": 1.0,
        "background_blur": False, "font_size": 18,
    }))
    runtime = base / "runtime"
    runtime.mkdir(mode=0o700)
    env = os.environ.copy()
    env.pop("WAYLAND_DISPLAY", None)
    env.update(
        XDG_CONFIG_HOME=str(base / "config"), XDG_DATA_HOME=str(base / "data"),
        XDG_CACHE_HOME=str(base / "cache"), XDG_RUNTIME_DIR=str(runtime),
        RUST_LOG=env.get("RUST_LOG", "elyterminal=debug,gpui_wgpu=info"),
    )
    log_path = base / "app.log"
    with log_path.open("w") as log:
        process = subprocess.Popen(sys.argv[1:], env=env, stdout=log,
                                   stderr=subprocess.STDOUT, start_new_session=True)
        try:
            deadline = time.monotonic() + 20
            window_id = None
            while time.monotonic() < deadline and process.poll() is None:
                if proof.exists():
                    window_id = app_window(process.pid)
                    if window_id and painted(window_id, base):
                        break
                time.sleep(0.25)
            else:
                raise RuntimeError("linux-gui-test: no painted window with a running PTY")
            if proof.read_text().strip() != env.get("LD_LIBRARY_PATH", "unset"):
                raise RuntimeError("linux-gui-test: library path leaked into the shell")
            print(output(["xprop", "-id", window_id, "WM_CLASS", "_NET_WM_NAME"]).strip())
            result = process.wait(timeout=35)
            if result:
                raise RuntimeError(f"linux-gui-test: application exited {result}")
            print(f"PASSED: X11 window, painted frame, PTY, child environment, exit 0; {base}")
        except BaseException:
            print(log_path.read_text(), file=sys.stderr)
            raise
        finally:
            if process.poll() is None:
                os.killpg(process.pid, signal.SIGTERM)
                process.wait(timeout=10)


if __name__ == "__main__":
    main()
