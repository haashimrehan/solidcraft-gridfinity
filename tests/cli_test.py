#!/usr/bin/env python3
"""End to end test: runs the built plugin through solidcraft-cli and measures what it made.

    SOLIDCRAFT_CLI=/path/to/solidcraft-cli python tests/cli_test.py [package-dir]

The package dir holds plugin.json and plugin.wasm (default: dist/solidcraft-gridfinity).
Needs a solidcraft-cli with the plugin host (SolidCraft 0.41 or newer).

Every volume is compared with a number worked out here from the Gridfinity dimensions, not with
a number the plugin reports.
"""
import json, math, os, shutil, subprocess, sys, tempfile, time

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CLI = os.environ.get("SOLIDCRAFT_CLI", "solidcraft-cli")
PKG = os.path.abspath(sys.argv[1] if len(sys.argv) > 1 else os.path.join(ROOT, "dist", "solidcraft-gridfinity"))
TMP = tempfile.mkdtemp(prefix="gridfinity-test-")
KEEP = os.environ.get("KEEP_DESIGNS")  # a folder to keep the saved designs in


def run(steps, save=None, inp=None, fail_ok=False):
    script = os.path.join(TMP, "s.json")
    out = os.path.join(TMP, "o.json")
    json.dump(steps, open(script, "w"))
    cmd = [CLI, "run", script, "--results", "--quiet", "--json-out", out]
    if inp:
        cmd += ["--in", inp]
    if save:
        cmd += ["--save", save]
    p = subprocess.run(cmd, capture_output=True, text=True)
    if p.returncode != 0:
        if fail_ok:
            return p.stderr + p.stdout
        sys.exit("cli failed: " + p.stderr[-2000:] + p.stdout[-500:])
    return [r.get("result", r) if isinstance(r, dict) else r for r in json.load(open(out))]


APPROVAL = run([{"command": "plugin.inspect", "params": {"path": PKG}}])[0]["approval"]


def plugin(entry, params):
    return {"command": "plugin.run", "params": {"path": PKG, "approval": APPROVAL, "entry": entry, "params": params}}


def check(name, ok, detail=""):
    print(("PASS " if ok else "FAIL ") + name + (" " + detail if detail else ""))
    if not ok:
        check.failed = True


check.failed = False


# --- the expected geometry, from the Gridfinity dimensions ---

def area(hx, hy, r):
    """A rounded rectangle with half sizes hx, hy and corner radius r."""
    return 4 * hx * hy - (4 - math.pi) * r * r


def taper(hx, hy, r, d):
    """Volume of a 45 degree extrude over d that starts at this outline and grows (d > 0) or
    shrinks (d < 0): Simpson's rule, exact because the area is quadratic in the offset."""
    a0, a1, a2 = area(hx, hy, r), area(hx + d / 2, hy + d / 2, r + d / 2), area(hx + d, hy + d, r + d)
    return abs(d) * (a0 + 4 * a1 + a2) / 6


FOOT = taper(17.8, 17.8, 0.8, 0.8) + area(18.6, 18.6, 1.6) * 1.8 + taper(18.6, 18.6, 1.6, 2.15)


MAGNET_SIZES = {"5x2": (5.5, 2.4), "6x2": (6.5, 2.4), "6x3": (6.5, 3.4), "8x2": (8.5, 2.4), "8x3": (8.5, 3.4)}


def magnet(p):
    """Pocket diameter and depth the parameters ask for."""
    size = p.get("magnet_size", "6x2")
    return (p.get("magnet_diameter", 6.5), p.get("magnet_depth", 2.4)) if size == "custom" else MAGNET_SIZES[size]


def bin_volume(p):
    nx, ny, hu = p["width"], p["depth"], p["height"]
    t, lip, magnets = p.get("wall", 1.2), p.get("stacking_lip", True), p.get("magnet_holes", True)
    cx, cy = p.get("compartments_x", 1), p.get("compartments_y", 1)
    hx, hy, h = (nx * 42 - 0.5) / 2, (ny * 42 - 0.5) / 2, hu * 7
    top = h + (4.15 if lip else 0)
    v = nx * ny * FOOT + area(hx, hy, 3.75) * (top - 4.75)
    if magnets:
        md, mh = magnet(p)
        v -= nx * ny * 4 * math.pi * (md / 2) ** 2 * mh
    if lip:
        s = 2.6 - t
        v -= taper(hx - 1.9, hy - 1.9, 1.85, 1.65)  # the flare, up to the rim
        v -= area(hx - 1.9, hy - 1.9, 1.85) * 1.8
        v -= taper(hx - 2.6, hy - 2.6, 1.15, 0.7)
        v -= taper(hx - t, hy - t, 3.75 - t, -s)  # the support under the lip
        v -= area(hx - t, hy - t, 3.75 - t) * (h - s - 7)
    else:
        v -= area(hx - t, hy - t, 3.75 - t) * (h - 7)
        # dividers in a straight-walled cavity: full walls, less where they cross
        inner = h - 7
        v += (cx - 1) * t * (2 * hy - 2 * t) * inner + (cy - 1) * t * (2 * hx - 2 * t) * inner
        v -= (cx - 1) * (cy - 1) * t * t * inner
    return v, (2 * hx, 2 * hy, top)


def plate_volume(p):
    nx, ny = p["width"], p["depth"]
    pocket = taper(18.05, 18.05, 1.05, 0.7) + area(18.75, 18.75, 1.75) * 1.8 + taper(18.75, 18.75, 1.75, 2.15)
    md, mh = magnet(p)
    floor = mh + 0.6 if p.get("magnet_holes") else 0.0
    v = area(21 * nx, 21 * ny, 4) * (4.65 + floor) - nx * ny * pocket
    if floor:
        v -= nx * ny * 4 * math.pi * (md / 2) ** 2 * mh
    return v, (42 * nx, 42 * ny, 4.65 + floor)


def built(label, entry, p, expect=None, save=None):
    """Run an entry and check the body it made; returns (measure, seconds)."""
    t0 = time.time()
    res = run([plugin(entry, p), {"command": "inspect.measure", "params": {}}], save=save)
    secs = time.time() - t0
    bodies = res[1]["bodies"]
    body = bodies[0]
    check(f"{label}: one closed solid", len(bodies) == 1 and body["closed"], f"({secs:.1f} s)")
    check(f"{label}: body named as the reply says", body["name"] == res[0].get("body"), body["name"])
    if expect:
        vol, size = expect
        got = body["volume_mm3"]
        check(f"{label}: volume {vol:.1f} mm3", abs(got - vol) / vol < 2e-4, f"got {got:.1f}")
        bb = body["bbox"]
        dims = [bb["max"][i] - bb["min"][i] for i in range(3)]
        check(f"{label}: size {size[0]:g} x {size[1]:g} x {size[2]:g}", all(abs(a - b) < 1e-6 for a, b in zip(dims, size)), str([round(d, 4) for d in dims]))
        check(f"{label}: centred on the origin, standing on Z 0", abs(bb["min"][0] + bb["max"][0]) < 1e-6 and abs(bb["min"][1] + bb["max"][1]) < 1e-6 and abs(bb["min"][2]) < 1e-6)
    return body, secs


os.makedirs(os.path.join(TMP, "out"), exist_ok=True)

# Bins whose volume has a closed form.
for label, p in {
    "1x1x2 plain": dict(width=1, depth=1, height=2, stacking_lip=False, magnet_holes=False),
    "1x1x3 lip": dict(width=1, depth=1, height=3, magnet_holes=False),
    "2x1x3 default": dict(width=2, depth=1, height=3),
    "3x2x6 thick": dict(width=3, depth=2, height=6, wall=2.0),
    "2x2x4 dividers": dict(width=2, depth=2, height=4, stacking_lip=False, compartments_x=3, compartments_y=2),
    "1x4x2": dict(width=1, depth=4, height=2),
    "2x1x3 8x3 magnets": dict(width=2, depth=1, height=3, magnet_size="8x3"),
    "1x1x3 small magnets": dict(width=1, depth=1, height=3, magnet_size="custom", magnet_diameter=3.2, magnet_depth=1.2),
}.items():
    sc = os.path.join(TMP, "out", label.split()[0] + ".sc")
    built(label, "make-bin", p, bin_volume(p), save=sc)
    reopened = run([{"command": "inspect.measure", "params": {}}], inp=sc)[0]
    check(f"{label}: saved .sc reopens with one closed body", len(reopened["bodies"]) == 1 and reopened["bodies"][0]["closed"])

# Magnet pockets: where they are and how big.
res = run([plugin("make-bin", dict(width=2, depth=1, height=3)), {"command": "model.faces", "params": {"body": "Gridfinity bin 2x1x3"}}])
faces = res[1]["faces"] if isinstance(res[1], dict) else res[1]
pockets = {(round(f["cylinder"]["origin"][0], 3), round(f["cylinder"]["origin"][1], 3)) for f in faces if f.get("cylinder") and abs(f["cylinder"]["radius"] - 3.25) < 1e-6}
want = {(cx + sx, sy) for cx in (-21, 21) for sx in (-13, 13) for sy in (-13, 13)}
check("2x1x3: 8 magnet pockets, 13 mm from each cell centre", pockets == want, str(sorted(pockets)))

# Scoop, label tab and dividers under a lip: one closed solid, each adding material.
base = dict(width=2, depth=1, height=4)
plain, _ = built("2x1x4 plain", "make-bin", base, bin_volume(base))
for label, extra in {
    "scoop": dict(scoop=True),
    "label tab": dict(label_tab=True),
    "dividers under a lip": dict(compartments_x=4),
    "everything": dict(scoop=True, label_tab=True, compartments_x=3, compartments_y=2),
}.items():
    body, _ = built(f"2x1x4 {label}", "make-bin", {**base, **extra})
    check(f"2x1x4 {label}: adds material, same outside", body["volume_mm3"] > plain["volume_mm3"] + 50 and body["bbox"] == plain["bbox"], f"+{body['volume_mm3'] - plain['volume_mm3']:.0f} mm3")

# Nothing sticks out of the bin's outline, the rounded corners included: the part of a bin inside
# its outline is all of it.
for label, p in {
    "everything": {**base, "scoop": True, "label_tab": True, "compartments_x": 3, "compartments_y": 2},
    "thin wall": dict(width=1, depth=1, height=3, wall=0.8, scoop=True, label_tab=True),
    "thick wall, no lip": dict(width=2, depth=2, height=5, wall=2.4, stacking_lip=False, scoop=True, label_tab=True),
}.items():
    hx, hy = (p["width"] * 42 - 0.5) / 2, (p["depth"] * 42 - 0.5) / 2
    res = run([
        plugin("make-bin", p),
        {"command": "inspect.measure", "params": {}},
        {"command": "sketch.create", "params": {"plane": "XY", "offset": 4.75, "name": "Hull outline"}},
        {"command": "sketch.rectangle.rounded", "params": {"p0": [-hx, -hy], "p1": [hx, hy], "radius": 3.75}},
        {"command": "sketch.finish", "params": {}},
        {"command": "solid.extrude", "params": {"sketch": "Hull outline", "distance": 200, "operation": "new", "name": "Hull", "body_name": "Hull"}},
        {"command": "solid.box", "params": {"length": 2 * hx, "width": 2 * hy, "height": 4.75, "center": [0, 0, 2.375], "operation": "join", "targets": ["Hull"], "name": "Hull base"}},
        {"command": "solid.combine", "params": {"target": "Hull", "tools": [res_name := f"Gridfinity bin {p['width']}x{p['depth']}x{p['height']}"], "operation": "intersect", "keep_tools": True}},
        {"command": "inspect.measure", "params": {}},
    ])
    whole = next(b for b in res[1]["bodies"] if b["name"] == res_name)["volume_mm3"]
    inside = sum(b["volume_mm3"] for b in res[-1]["bodies"] if b["name"].startswith("Hull"))
    check(f"inside the outline, {label}", len(res[1]["bodies"]) == 1 and abs(whole - inside) < 0.01, f"{whole - inside:.3f} mm3 outside")

# The scoop's volume: a square less a quarter disc, along the inside (the rounded corners of the
# cavity take a little off each end).
t, r = 1.2, 12
scoop, _ = built("2x1x4 scoop, no lip", "make-bin", {**base, "stacking_lip": False, "scoop": True})
flat, _ = built("2x1x4 no lip", "make-bin", {**base, "stacking_lip": False})
added, ideal = scoop["volume_mm3"] - flat["volume_mm3"], (r * r - math.pi * r * r / 4) * (83.5 - 2 * t)
check(f"scoop: about {ideal:.0f} mm3 of ramp", 0.97 * ideal < added <= ideal + 1, f"got {added:.0f}")

# Baseplates.
for label, p in {
    "1x1": dict(width=1, depth=1),
    "3x2": dict(width=3, depth=2),
    "4x3": dict(width=4, depth=3),
    "1x1 magnets": dict(width=1, depth=1, magnet_holes=True),
    "3x2 magnets": dict(width=3, depth=2, magnet_holes=True),
    "2x2 8x3 magnets": dict(width=2, depth=2, magnet_holes=True, magnet_size="8x3"),
}.items():
    built(f"baseplate {label}", "make-baseplate", p, plate_volume(p), save=os.path.join(TMP, "out", f"plate-{label.replace(' ', '-')}.sc"))

# Baseplate magnet pockets: under those of a bin, opening into the pocket floor.
res = run([plugin("make-baseplate", dict(width=2, depth=1, magnet_holes=True)), {"command": "model.faces", "params": {"body": "Gridfinity baseplate 2x1 magnets"}}])
faces = res[1]["faces"] if isinstance(res[1], dict) else res[1]
pockets = {(round(f["cylinder"]["origin"][0], 3), round(f["cylinder"]["origin"][1], 3)) for f in faces if f.get("cylinder") and abs(f["cylinder"]["radius"] - 3.25) < 1e-6}
check("2x1 magnet baseplate: 8 magnet pockets, 13 mm from each cell centre", pockets == want, str(sorted(pockets)))
floors = {round(f["centroid"][2], 3) for f in faces if f.get("plane_normal") and f["plane_normal"][2] > 0.999}
check("2x1 magnet baseplate: pockets 2.4 mm deep under a floor at 3 mm", {0.6, 3.0, 7.65} <= floors, str(sorted(floors)))

# The largest sizes stay well inside the host's 30 second limit.
big = dict(width=5, depth=5, height=3)
for label, entry, p, expect in [
    ("5x5x3 bin", "make-bin", big, bin_volume(big)),
    ("6x4 bin, 8x8 compartments", "make-bin", dict(width=6, depth=4, height=6, compartments_x=8, compartments_y=8, scoop=True, label_tab=True), None),
    ("6x6 baseplate", "make-baseplate", dict(width=6, depth=6), plate_volume(dict(width=6, depth=6))),
    ("6x6 magnet baseplate", "make-baseplate", dict(width=6, depth=6, magnet_holes=True), plate_volume(dict(width=6, depth=6, magnet_holes=True))),
]:
    _, secs = built(label, entry, p, expect)
    check(f"{label}: under 20 s", secs < 20, f"{secs:.1f} s")

# Two runs in one design: the second bin gets its own body and does not disturb the first.
res = run([plugin("make-bin", dict(width=1, depth=1, height=2)), plugin("make-bin", dict(width=1, depth=1, height=2)), {"command": "inspect.measure", "params": {}}])
names = [b["name"] for b in res[2]["bodies"]]
check("two runs: two bodies with different names", len(names) == 2 and len(set(names)) == 2 and res[1]["body"] == names[1], str(names))
check("two runs: both the same closed solid", all(b["closed"] and abs(b["volume_mm3"] - res[2]["bodies"][0]["volume_mm3"]) < 1e-3 for b in res[2]["bodies"]))

# Refused inputs name the field.
for label, entry, p, field in [
    ("width 0", "make-bin", dict(width=0), "width"),
    ("height 1", "make-bin", dict(height=1), "height"),
    ("wall 5", "make-bin", dict(wall=5), "wall"),
    ("too many compartments", "make-bin", dict(width=1, compartments_x=8), "compartments_x"),
    ("6x6 bin", "make-bin", dict(width=6, depth=6), "cells"),
    ("baseplate depth 7", "make-baseplate", dict(depth=7), "depth"),
    ("magnet diameter 9", "make-bin", dict(magnet_size="custom", magnet_diameter=9), "magnet_diameter"),
    ("magnet size 10x2", "make-bin", dict(magnet_size="10x2"), "magnet_size"),
    ("magnet depth 5", "make-baseplate", dict(magnet_holes=True, magnet_size="custom", magnet_depth=5), "magnet_depth"),
]:
    out = run([plugin(entry, p)], fail_ok=True)
    check(f"refused: {label}", isinstance(out, str) and field in out, out.strip()[-160:] if isinstance(out, str) else "it ran")

if KEEP:
    shutil.copytree(os.path.join(TMP, "out"), KEEP, dirs_exist_ok=True)
shutil.rmtree(TMP)
sys.exit(1 if check.failed else 0)
