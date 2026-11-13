#!/usr/bin/env python3
"""Record the demo: drive the SolidCraft window through the Plugins menu while ffmpeg captures it.

    python demo/record_demo.py [out.mp4] [--dry]

Needs SolidCraft running with a control port (SOLIDCRAFT_CONTROL_PORT=7878), maximised and in
front, the plugin installed, ffmpeg on PATH, and sc.py from the SolidCraft agent skill
(SC_PY points at it). `--dry` runs the moves without recording.
"""
import importlib.util, math, os, subprocess, sys, time

PORT = int(os.environ.get("SOLIDCRAFT_CONTROL_PORT", "7878"))
SC_PY = os.environ.get("SC_PY", os.path.expanduser("~/.claude/skills/solidcraft/scripts/sc.py"))
OUT = next((a for a in sys.argv[1:] if not a.startswith("--")), "demo/raw.mp4")
DRY = "--dry" in sys.argv
# The captured part of the desktop, in physical pixels: the maximised window without the taskbar.
SIZE = os.environ.get("CAPTURE_SIZE", "1920x1140")

spec = importlib.util.spec_from_file_location("sc", SC_PY)
sc = importlib.util.module_from_spec(spec)
spec.loader.exec_module(sc)
c = sc.Conn(PORT)
T0 = time.time()
MARKS = []


def mark(name):
    """Note when a moment happens, in seconds of the recording, for the edit."""
    MARKS.append((round(time.time() - T0, 2), name))
    print(f"{MARKS[-1][0]:7.2f}  {name}", flush=True)


def call(method, params=None):
    return c.request(method, params or {})


def execute(command, params=None):
    return call("engine.execute", {"command": command, "params": params or {}})


def handle(name):
    for h in call("ui.handles"):
        if h["name"] == name:
            return h["x"], h["y"]
    raise SystemExit(f"no handle {name}")


def glide(x, y, secs=0.45):
    """Move the pointer there in small steps, so the cursor travels instead of jumping."""
    steps = max(2, int(secs * 30))
    x0, y0 = glide.at
    for i in range(1, steps + 1):
        t = i / steps
        t = t * t * (3 - 2 * t)
        call("ui.move", {"x": x0 + (x - x0) * t, "y": y0 + (y - y0) * t})
        time.sleep(secs / steps)
    glide.at = (x, y)


glide.at = (760.0, 480.0)


def click(x, y, pause=0.45):
    glide(x, y)
    time.sleep(0.12)
    call("ui.click", {"x": x, "y": y})
    time.sleep(pause)


def click_handle(name, pause=0.4):
    click(*handle(name), pause=pause)


def dialog_open():
    """Whether a plugin entry's dialog is up (ui.inspect's `dialog` does not report it)."""
    return any(h["name"] == "plugin_dialog:run" for h in call("ui.handles"))


def menu_open():
    return any(h["name"].startswith("menu:") for h in call("ui.handles"))


def open_entry(row, label):
    """Plugins > Gridfinity > the row whose label starts with `row`."""
    if not menu_open():
        click(*handle("appbar:plugins"), pause=0.6)
    click(230, 98, pause=0.6)
    at = next(((h["x"], h["y"]) for h in call("ui.handles") if h["name"].startswith("menu:" + row)), None)
    if not at:
        raise SystemExit(f"no menu row {row}")
    click(at[0] + 10, at[1], pause=0.3)
    for _ in range(40):
        if dialog_open():
            break
        time.sleep(0.1)
    else:
        raise SystemExit(f"the {label} dialog did not open")
    mark(f"{label} dialog open")
    time.sleep(0.9)


def run_and_wait(label, limit=40):
    click_handle("plugin_dialog:run", pause=0.1)
    mark(f"{label} run clicked")
    t = time.time()
    while dialog_open():
        if time.time() - t > limit:
            raise SystemExit(f"{label} did not finish in {limit} s")
        time.sleep(0.1)
    mark(f"{label} built")


def camera():
    return call("ui.inspect")["camera"]


def fly(to, secs):
    """Ease the camera from where it is to `to` (target, yaw, pitch, distance)."""
    a = camera()
    steps = max(2, int(secs * 30))
    for i in range(1, steps + 1):
        t = i / steps
        t = t * t * (3 - 2 * t)
        mix = lambda k: a[k] + (to[k] - a[k]) * t
        call("ui.camera", {"target": [a["target"][j] + (to["target"][j] - a["target"][j]) * t for j in range(3)],
                           "yaw": mix("yaw"), "pitch": mix("pitch"), "distance": mix("distance")})
        time.sleep(secs / steps)


# --- a clean stage ---
if menu_open():  # the Plugins button toggles its menu
    call("ui.click", dict(zip("xy", handle("appbar:plugins"))))
    time.sleep(0.4)
if dialog_open():
    call("ui.click", dict(zip("xy", handle("plugin_dialog:cancel"))))
    time.sleep(0.4)
call("ui.key", {"key": "Escape"})
execute("file.new")
call("ui.agent_cursor", {"show": True})
home = {"target": [0.0, 0.0, 0.0], "yaw": -0.785, "pitch": 0.615, "distance": 300.0}
call("ui.camera", home)
call("ui.move", {"x": glide.at[0], "y": glide.at[1]})
time.sleep(0.8)

rec = None
if not DRY:
    os.makedirs(os.path.dirname(OUT) or ".", exist_ok=True)
    rec = subprocess.Popen(
        ["ffmpeg", "-hide_banner", "-loglevel", "error", "-y", "-f", "gdigrab", "-framerate", "30", "-draw_mouse", "0",
         "-offset_x", "0", "-offset_y", "0", "-video_size", SIZE, "-i", "desktop",
         "-c:v", "libx264", "-preset", "veryfast", "-crf", "16", "-pix_fmt", "yuv420p", OUT],
        stdin=subprocess.PIPE)
    time.sleep(1.2)
T0 = time.time()
mark("start")
time.sleep(0.8)

try:
    # 1. A 3 x 2 baseplate, with the dialog's defaults.
    open_entry("Baseplate", "baseplate")
    run_and_wait("baseplate")
    time.sleep(0.5)
    fly({"target": [0.0, 0.0, 2.0], "yaw": -0.6, "pitch": 0.75, "distance": 250.0}, 1.4)
    mark("baseplate framed")
    time.sleep(0.9)

    # 2. A 3 x 2 x 4 bin with three compartments, a scoop and a label tab.
    open_entry("Bin", "bin")
    click_handle("dialog:Width * up", 0.35)
    click_handle("dialog:Depth * up", 0.35)
    click_handle("dialog:Height * up", 0.35)
    click_handle("dialog:Compartments along X * up", 0.3)
    click_handle("dialog:Compartments along X * up", 0.35)
    click_handle("plugin_dialog:field:scoop", 0.35)
    click_handle("plugin_dialog:field:label_tab", 0.6)
    mark("bin dialog filled")
    run_and_wait("bin")
    glide(1180, 700, 0.5)
    time.sleep(0.4)

    # 3. Look at it: frame it, go once round, then down into the compartments.
    start = {"target": [0.0, 0.0, 14.0], "yaw": -0.6, "pitch": 0.6, "distance": 250.0}
    fly(start, 1.2)
    mark("bin framed")
    time.sleep(0.6)
    steps = 30 * 6
    for i in range(1, steps + 1):
        t = i / steps
        ease = t * t * (3 - 2 * t)
        call("ui.camera", {"yaw": start["yaw"] + 2 * math.pi * ease, "pitch": start["pitch"] + 0.25 * math.sin(math.pi * t)})
        time.sleep(1 / 30)
    mark("orbit done")
    top = dict(start)
    top["yaw"] = start["yaw"] + 2 * math.pi + 0.35
    top["pitch"] = 0.95
    top["distance"] = 215.0
    fly(top, 1.6)
    mark("top view")
    time.sleep(1.8)
    mark("end")
finally:
    if rec:
        rec.stdin.write(b"q")
        rec.stdin.flush()
        rec.wait(timeout=20)
    call("ui.agent_cursor", {"show": False})

with open(os.path.splitext(OUT)[0] + ".marks.txt", "w") as f:
    for t, name in MARKS:
        f.write(f"{t:.2f}\t{name}\n")
