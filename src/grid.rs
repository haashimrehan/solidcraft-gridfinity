//! Gridfinity geometry, built from ordinary SolidCraft commands.
//!
//! Dimensions follow the Gridfinity specification (Zack Freedman, MIT): a 42 mm grid, 7 mm height
//! units, feet with a 0.8 / 1.8 / 2.15 mm profile and a stacking lip with a 0.7 / 1.8 / 1.9 mm
//! profile. Every tapered surface is a 45 degree extrude of a rounded rectangle, so the result is
//! a short timeline of sketches and extrudes that stays editable.

use crate::Runner;
use serde_json::{json, Value};

/// Grid pitch.
pub const PITCH: f64 = 42.0;
/// The largest grid along one side: 252 mm, what a 256 mm printer bed takes.
pub const MAX_GRID: u32 = 6;
/// The most cells a bin may have. The host gives a plugin 30 seconds, and joining the feet takes
/// longer than linearly in their number (25 cells take about 10 s on a 2024 desktop).
pub const MAX_BIN_CELLS: u32 = 25;
/// One height unit.
pub const UNIT_H: f64 = 7.0;
/// Gap between a bin and its grid cell, in total (0.25 mm a side).
pub const CLEARANCE: f64 = 0.5;
/// Outer corner radius of a bin.
pub const CORNER_R: f64 = 3.75;
/// Foot profile, bottom to top: a 45 degree chamfer, a straight wall, a 45 degree flare.
pub const FOOT_CHAMFER: f64 = 0.8;
pub const FOOT_WALL: f64 = 1.8;
pub const FOOT_FLARE: f64 = 2.15;
/// Foot height: where the bin body starts.
pub const FOOT_H: f64 = FOOT_CHAMFER + FOOT_WALL + FOOT_FLARE;
/// Stacking lip profile, bottom to top.
pub const LIP_CHAMFER: f64 = 0.7;
pub const LIP_WALL: f64 = 1.8;
pub const LIP_FLARE: f64 = 1.9;
/// How far the lip reaches in from the outer wall at its base.
pub const LIP_REACH: f64 = LIP_CHAMFER + LIP_FLARE;
/// Flat left on top of the lip instead of a knife edge.
pub const LIP_LAND: f64 = 0.25;
/// Lip height above the bin body.
pub const LIP_H: f64 = LIP_CHAMFER + LIP_WALL + LIP_FLARE - LIP_LAND;
/// Top of the bin floor: one height unit.
pub const FLOOR_Z: f64 = 7.0;
/// Magnet pocket: 6 mm magnets, press fit.
pub const MAGNET_D: f64 = 6.5;
pub const MAGNET_DEPTH: f64 = 2.4;
/// Magnet centres from the cell centre.
pub const MAGNET_OFFSET: f64 = 13.0;
/// Room left under the lip for the feet of the bin stacked on top.
pub const STACK_ROOM: f64 = 1.0;
/// Baseplate profile, bottom to top, and its height.
pub const PLATE_CHAMFER: f64 = 0.7;
pub const PLATE_WALL: f64 = 1.8;
pub const PLATE_FLARE: f64 = 2.15;
pub const PLATE_H: f64 = PLATE_CHAMFER + PLATE_WALL + PLATE_FLARE;
/// Corner radius of a baseplate.
pub const PLATE_R: f64 = 4.0;
/// Flat left on top of each baseplate ridge instead of a knife edge.
pub const PLATE_LAND: f64 = 0.1;
/// Solid floor under the pockets of a magnet baseplate: a magnet pocket and a 0.6 mm skin
/// under it, so the magnets sit flush with the floor, right under those of the bin.
pub const PLATE_FLOOR: f64 = MAGNET_DEPTH + 0.6;

#[derive(Clone, Debug, PartialEq)]
pub struct Bin {
    pub width: u32,
    pub depth: u32,
    pub height: u32,
    pub wall: f64,
    pub lip: bool,
    pub magnets: bool,
    pub compartments_x: u32,
    pub compartments_y: u32,
    pub scoop: bool,
    pub label: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Baseplate {
    pub width: u32,
    pub depth: u32,
    pub magnets: bool,
}

pub type Invalid = (String, String);

fn bad(field: &str, why: impl Into<String>) -> Invalid {
    (field.to_owned(), why.into())
}

impl Bin {
    pub fn from_json(v: &Value) -> Self {
        let int = |k: &str, d: u32| v.get(k).and_then(Value::as_f64).map_or(d, |n| n.round().max(0.0) as u32);
        let flag = |k: &str, d: bool| v.get(k).and_then(Value::as_bool).unwrap_or(d);
        Bin {
            width: int("width", 2),
            depth: int("depth", 1),
            height: int("height", 3),
            wall: v.get("wall").and_then(Value::as_f64).unwrap_or(1.2),
            lip: flag("stacking_lip", true),
            magnets: flag("magnet_holes", true),
            compartments_x: int("compartments_x", 1),
            compartments_y: int("compartments_y", 1),
            scoop: flag("scoop", false),
            label: flag("label_tab", false),
        }
    }

    /// Outer length along X and Y, and the body height without the lip.
    pub fn size(&self) -> (f64, f64, f64) {
        (
            self.width as f64 * PITCH - CLEARANCE,
            self.depth as f64 * PITCH - CLEARANCE,
            self.height as f64 * UNIT_H,
        )
    }

    /// Top of the walls: the lip's base with a lip, the rim without.
    fn inner_top(&self) -> f64 {
        let h = self.size().2;
        if self.lip {
            h - STACK_ROOM
        } else {
            h
        }
    }

    pub fn validate(&self) -> Result<(), Invalid> {
        if !(1..=MAX_GRID).contains(&self.width) {
            return Err(bad("width", "1 to 6 grid units"));
        }
        if !(1..=MAX_GRID).contains(&self.depth) {
            return Err(bad("depth", "1 to 6 grid units"));
        }
        if self.width * self.depth > MAX_BIN_CELLS {
            return Err(bad("depth", format!("{} x {} is more than {MAX_BIN_CELLS} cells, too many to build in the 30 seconds a plugin gets", self.width, self.depth)));
        }
        if !(2..=20).contains(&self.height) {
            return Err(bad("height", "2 to 20 height units of 7 mm"));
        }
        if !(self.wall >= 0.8 && self.wall <= 2.4) {
            return Err(bad("wall", "0.8 to 2.4 mm"));
        }
        let (l, w, _) = self.size();
        for (field, count, span) in [("compartments_x", self.compartments_x, l), ("compartments_y", self.compartments_y, w)] {
            if !(1..=8).contains(&count) {
                return Err(bad(field, "1 to 8"));
            }
            let room = (span - 2.0 * self.wall) / count as f64 - self.wall;
            if room < 5.0 {
                return Err(bad(field, format!("each compartment would be {room:.1} mm wide; use fewer")));
            }
        }
        Ok(())
    }
}

impl Baseplate {
    pub fn from_json(v: &Value) -> Self {
        let int = |k: &str, d: u32| v.get(k).and_then(Value::as_f64).map_or(d, |n| n.round().max(0.0) as u32);
        let magnets = v.get("magnet_holes").and_then(Value::as_bool).unwrap_or(false);
        Baseplate { width: int("width", 3), depth: int("depth", 2), magnets }
    }

    /// Height of the floor under the pockets: none for a plain frame.
    pub fn floor(&self) -> f64 {
        if self.magnets {
            PLATE_FLOOR
        } else {
            0.0
        }
    }

    pub fn validate(&self) -> Result<(), Invalid> {
        if !(1..=MAX_GRID).contains(&self.width) {
            return Err(bad("width", "1 to 6 grid units"));
        }
        if !(1..=MAX_GRID).contains(&self.depth) {
            return Err(bad("depth", "1 to 6 grid units"));
        }
        Ok(())
    }
}

/// The commands of one build, with the body the next feature works on.
///
/// Sketches are referred to by the id `sketch.create` returns and bodies by the name the reply
/// gives, so a second run in the same design, where the names are taken, still works on its own.
struct Build<'a, R: Runner> {
    r: &'a mut R,
    body: String,
}

impl<R: Runner> Build<'_, R> {
    /// A sketch on the XY plane at height `z` holding one rounded rectangle; returns its id.
    fn outline(&mut self, name: &str, z: f64, c: [f64; 2], half: [f64; 2], radius: f64) -> Result<Value, String> {
        let made = self.r.run("sketch.create", json!({"plane": "XY", "offset": z, "name": name}))?;
        self.r.run(
            "sketch.rectangle.rounded",
            json!({"p0": [c[0] - half[0], c[1] - half[1]], "p1": [c[0] + half[0], c[1] + half[1]], "radius": radius}),
        )?;
        self.r.run("sketch.finish", json!({}))?;
        Ok(made.get("sketch").cloned().unwrap_or_else(|| json!(name)))
    }

    /// Extrude `sketch` from `start` above its plane over `distance`; `taper` in degrees, positive
    /// growing outward. `new: Some(name)` starts a body of that name, which later features then
    /// work on.
    fn extrude(&mut self, sketch: &Value, name: &str, start: f64, distance: f64, taper: f64, operation: &str, new: Option<&str>) -> Result<(), String> {
        let mut p = json!({"sketch": sketch, "distance": distance, "name": name});
        if start != 0.0 {
            p["start_offset"] = json!(start);
        }
        if taper != 0.0 {
            p["taper"] = json!(format!("{taper} deg"));
        }
        match new {
            Some(body) => {
                p["operation"] = json!("new");
                p["body_name"] = json!(body);
            }
            None => {
                p["operation"] = json!(operation);
                p["targets"] = json!([self.body]);
            }
        }
        let made = self.r.run("solid.extrude", p)?;
        if let Some(body) = new {
            self.body = created(&made).into_iter().next().unwrap_or_else(|| body.to_owned());
        }
        Ok(())
    }

    /// Copy the current body `count - 1` times along `dir` at the grid pitch; returns the copies.
    fn copies(&mut self, name: &str, dir: [i32; 3], count: u32) -> Result<Vec<String>, String> {
        if count < 2 {
            return Ok(Vec::new());
        }
        let made = self.r.run(
            "solid.pattern.rectangular",
            json!({"bodies": [self.body], "dir1": dir, "count1": count, "spacing1": PITCH, "name": name}),
        )?;
        let copies = created(&made);
        if copies.len() != count as usize - 1 {
            return Err(format!("{name}: expected {} copies, the pattern made {}", count - 1, copies.len()));
        }
        Ok(copies)
    }

    /// Copy the current body along `dir` and join the copies into it, so it stays one body.
    /// Joining a row first and then the rows keeps each boolean small: the kernel slows down as
    /// a body gains faces.
    fn repeat(&mut self, name: &str, dir: [i32; 3], count: u32, touching: Option<&str>) -> Result<(), String> {
        let copies = self.copies(name, dir, count)?;
        if copies.is_empty() {
            return Ok(());
        }
        let mut p = json!({"target": self.body, "tools": copies, "operation": "join", "name": format!("Join {}", name.to_lowercase())});
        if let Some(rule) = touching {
            p["touching"] = json!(rule);
        }
        self.r.run("solid.combine", p)?;
        Ok(())
    }

    /// A closed profile on the YZ plane (sketch x is world Y, sketch y is world Z), extruded
    /// `length` along X about the middle of the bin and joined to it.
    fn side_profile(&mut self, name: &str, segments: &[Seg], length: f64) -> Result<(), String> {
        let made = self.r.run("sketch.create", json!({"plane": "YZ", "name": format!("{name} profile")}))?;
        for s in segments {
            match s {
                Seg::Line(a, b) => self.r.run("sketch.line", json!({"points": [a, b]}))?,
                Seg::Arc(a, b, through) => self.r.run("sketch.arc.three_point", json!({"start": a, "end": b, "through": through}))?,
            };
        }
        self.r.run("sketch.finish", json!({}))?;
        let sketch = made.get("sketch").cloned().unwrap_or(Value::Null);
        self.r.run(
            "solid.extrude",
            json!({"sketch": sketch, "distance": length, "direction": "symmetric", "operation": "join", "targets": [self.body], "name": name}),
        )?;
        Ok(())
    }
}

/// The bodies a feature's reply says it created.
fn created(reply: &Value) -> Vec<String> {
    reply
        .get("created_bodies")
        .and_then(Value::as_array)
        .map(|b| b.iter().filter_map(Value::as_str).map(str::to_owned).collect())
        .unwrap_or_default()
}

enum Seg {
    Line([f64; 2], [f64; 2]),
    Arc([f64; 2], [f64; 2], [f64; 2]),
}

/// Build a bin centred on the origin, standing on Z 0.
///
/// The inside is cut while the body is still a plain block, one foot is built on its own and
/// copied, and the feet join the body last: the kernel's booleans slow down as a body gains
/// faces, so the many feet meet the body once, at the end.
pub fn make_bin<R: Runner>(r: &mut R, b: &Bin) -> Result<Value, String> {
    b.validate().map_err(|(f, why)| format!("{f}: {why}"))?;
    let (l, w, h) = b.size();
    let t = b.wall;
    let top = if b.lip { h + LIP_H } else { h };
    let half = [l / 2.0, w / 2.0];
    let inset = |d: f64| [half[0] - d, half[1] - d];
    let name = format!("Gridfinity bin {}x{}x{}", b.width, b.depth, b.height);
    let mut k = Build { r, body: String::new() };

    // The body, from the top of the feet to the rim.
    let outline = k.outline("Bin outline", FOOT_H, [0.0, 0.0], half, CORNER_R)?;
    k.extrude(&outline, "Bin body", 0.0, top - FOOT_H, 0.0, "new", Some(&name))?;
    let bin = k.body.clone();

    // The inside, top down: the stacking lip, the 45 degree support under it, then the cavity.
    let cavity = k.outline("Cavity outline", FLOOR_Z, [0.0, 0.0], inset(t), CORNER_R - t)?;
    if b.lip {
        let lip_mid = k.outline("Lip profile", h + LIP_CHAMFER, [0.0, 0.0], inset(LIP_FLARE), CORNER_R - LIP_FLARE)?;
        // The flare runs past the rim, so the cut ends in air and leaves the land on top.
        k.extrude(&lip_mid, "Lip flare", LIP_WALL, LIP_FLARE, 45.0, "cut", None)?;
        k.extrude(&lip_mid, "Lip wall", 0.0, LIP_WALL, 0.0, "cut", None)?;
        let lip_base = k.outline("Lip base", h, [0.0, 0.0], inset(LIP_REACH), CORNER_R - LIP_REACH)?;
        k.extrude(&lip_base, "Lip chamfer", 0.0, LIP_CHAMFER, 45.0, "cut", None)?;
        let support = LIP_REACH - t;
        k.extrude(&cavity, "Lip support", h - support - FLOOR_Z, support, -45.0, "cut", None)?;
        k.extrude(&cavity, "Cavity", 0.0, h - support - FLOOR_Z, 0.0, "cut", None)?;
    } else {
        k.extrude(&cavity, "Cavity", 0.0, h - FLOOR_Z + 1.0, 0.0, "cut", None)?;
    }

    // Dividers: plain walls sunk half a millimetre into the floor and half a wall into the sides,
    // all those of one direction drawn in one sketch and joined by one extrude.
    let wall_top = b.inner_top();
    let sunk = FLOOR_Z - 0.5;
    for (count, along_x) in [(b.compartments_x, true), (b.compartments_y, false)] {
        if count < 2 {
            continue;
        }
        let axis = if along_x { "X" } else { "Y" };
        let span = (if along_x { l } else { w }) - 2.0 * t;
        let made = k.r.run("sketch.create", json!({"plane": "XY", "offset": sunk, "name": format!("Divider lines along {axis}")}))?;
        for i in 1..count {
            let at = -span / 2.0 + span * i as f64 / count as f64;
            let (p0, p1) = if along_x {
                ([at - t / 2.0, -(w - t) / 2.0], [at + t / 2.0, (w - t) / 2.0])
            } else {
                ([-(l - t) / 2.0, at - t / 2.0], [(l - t) / 2.0, at + t / 2.0])
            };
            k.r.run("sketch.rectangle.two_point", json!({"p0": p0, "p1": p1}))?;
        }
        k.r.run("sketch.finish", json!({}))?;
        let sketch = made.get("sketch").cloned().unwrap_or(Value::Null);
        k.extrude(&sketch, &format!("Dividers along {axis}"), 0.0, wall_top - sunk, 0.0, "join", None)?;
    }

    let front = -w / 2.0 + t;
    let back = w / 2.0 - t;
    let clear = (if b.lip { h - (LIP_REACH - t) } else { h }) - FLOOR_Z;
    if b.label {
        // A shelf along the back wall with a 45 degree underside, so it prints without support.
        let reach = 12f64.min((w - 2.0 * t) * 0.4).min(clear - 1.0);
        let a = [back + t / 2.0, wall_top];
        let tip = [back - reach, wall_top];
        let foot = [back + t / 2.0, wall_top - reach - t / 2.0];
        k.side_profile("Label tab", &[Seg::Line(a, tip), Seg::Line(tip, foot), Seg::Line(foot, a)], l - t)?;
    }
    if b.scoop {
        // A concave ramp along the front wall, so small parts slide out under a finger.
        let radius = clear.min((w - 2.0 * t) / 2.0).min(12.0);
        let centre = [front + radius, FLOOR_Z + radius];
        let a = [front - t / 2.0, sunk];
        let floor_end = [front + radius, sunk];
        let arc_start = [front + radius, FLOOR_Z];
        let arc_end = [front, FLOOR_Z + radius];
        let through = [centre[0] - radius * std::f64::consts::FRAC_1_SQRT_2, centre[1] - radius * std::f64::consts::FRAC_1_SQRT_2];
        let wall_end = [front - t / 2.0, FLOOR_Z + radius];
        k.side_profile(
            "Scoop",
            &[
                Seg::Line(a, floor_end),
                Seg::Line(floor_end, arc_start),
                Seg::Arc(arc_start, arc_end, through),
                Seg::Line(arc_end, wall_end),
                Seg::Line(wall_end, a),
            ],
            l - t,
        )?;
    }

    if b.label || b.scoop {
        // Both run the full width, half a wall into the side walls, so their ends stick out of
        // the rounded outer corners: keep only what lies inside the bin's outline.
        k.extrude(&outline, "Trim to outline", 0.0, top - FOOT_H, 0.0, "intersect", None)?;
    }

    // One foot under the first cell, as a body of its own.
    let cell = [-(b.width as f64 - 1.0) * PITCH / 2.0, -(b.depth as f64 - 1.0) * PITCH / 2.0];
    let foot_top = (PITCH - CLEARANCE) / 2.0;
    let foot_mid = foot_top - FOOT_FLARE;
    let foot_base = foot_mid - FOOT_CHAMFER;
    let base = k.outline("Foot base", 0.0, cell, [foot_base; 2], CORNER_R - FOOT_FLARE - FOOT_CHAMFER)?;
    k.extrude(&base, "Foot chamfer", 0.0, FOOT_CHAMFER, 45.0, "new", Some(&format!("{name} foot")))?;
    let mid = k.outline("Foot profile", FOOT_CHAMFER, cell, [foot_mid; 2], CORNER_R - FOOT_FLARE)?;
    k.extrude(&mid, "Foot wall", 0.0, FOOT_WALL, 0.0, "join", None)?;
    k.extrude(&mid, "Foot flare", FOOT_WALL, FOOT_FLARE, 45.0, "join", None)?;
    if b.magnets {
        let made = k.r.run("sketch.create", json!({"plane": "XY", "name": "Magnet circles"}))?;
        for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
            k.r.run(
                "sketch.circle.center",
                json!({"center": [cell[0] + sx * MAGNET_OFFSET, cell[1] + sy * MAGNET_OFFSET], "radius": MAGNET_D / 2.0}),
            )?;
        }
        k.r.run("sketch.finish", json!({}))?;
        let circles = made.get("sketch").cloned().unwrap_or(Value::Null);
        k.extrude(&circles, "Magnet holes", 0.0, MAGNET_DEPTH, 0.0, "cut", None)?;
    }

    // A foot under every cell: a row of feet as one body (they do not touch each other), a copy
    // of the row under every row of cells, then the rows onto the body.
    k.repeat("Foot row", [1, 0, 0], b.width, Some("one_body"))?;
    let mut feet = vec![k.body.clone()];
    feet.extend(k.copies("Foot rows", [0, 1, 0], b.depth)?);
    k.r.run("solid.combine", json!({"target": bin, "tools": feet, "operation": "join", "name": "Join feet"}))?;

    Ok(json!({
        "body": bin,
        "grid": [b.width, b.depth, b.height],
        "size_mm": [l, w, top],
        "wall_mm": t,
        "stacking_lip": b.lip,
        "magnet_holes": if b.magnets { b.width * b.depth * 4 } else { 0 },
        "compartments": b.compartments_x * b.compartments_y,
    }))
}

/// Build a baseplate centred on the origin, standing on Z 0.
///
/// One cell is built as a square tile with its pocket, a row of tiles is joined, then the rows,
/// and the four outer corners are rounded last, by the plate's outline. With magnet holes the
/// pockets stand on a solid floor with a magnet pocket under each corner of every cell.
pub fn make_baseplate<R: Runner>(r: &mut R, p: &Baseplate) -> Result<Value, String> {
    p.validate().map_err(|(f, why)| format!("{f}: {why}"))?;
    let (l, w) = (p.width as f64 * PITCH, p.depth as f64 * PITCH);
    let floor = p.floor();
    let height = floor + PLATE_H;
    let name = format!("Gridfinity baseplate {}x{}{}", p.width, p.depth, if p.magnets { " magnets" } else { "" });
    let cell = [-(p.width as f64 - 1.0) * PITCH / 2.0, -(p.depth as f64 - 1.0) * PITCH / 2.0];
    let made = r.run(
        "solid.box",
        json!({"length": PITCH, "width": PITCH, "height": height, "center": [cell[0], cell[1], height / 2.0], "operation": "new", "name": "Cell", "body_name": name}),
    )?;
    let plate = created(&made).into_iter().next().unwrap_or(name);
    let mut k = Build { r, body: plate.clone() };

    // The pocket: a 45 degree chamfer, a straight wall, then a 45 degree flare that runs past the
    // top, so the cut ends in air and leaves a small land on each ridge.
    let top = PITCH / 2.0 - PLATE_LAND;
    let mid = top - PLATE_FLARE;
    let base = mid - PLATE_CHAMFER;
    let radius = PLATE_R - PLATE_LAND;
    let base_sketch = k.outline("Pocket base", floor, cell, [base; 2], radius - PLATE_FLARE - PLATE_CHAMFER)?;
    k.extrude(&base_sketch, "Pocket chamfer", 0.0, PLATE_CHAMFER, 45.0, "cut", None)?;
    let mid_sketch = k.outline("Pocket profile", floor + PLATE_CHAMFER, cell, [mid; 2], radius - PLATE_FLARE)?;
    k.extrude(&mid_sketch, "Pocket wall", 0.0, PLATE_WALL, 0.0, "cut", None)?;
    k.extrude(&mid_sketch, "Pocket flare", PLATE_WALL, PLATE_FLARE + 0.5, 45.0, "cut", None)?;

    // Magnet pockets open into the pocket floor, right under the bin's own magnets.
    if p.magnets {
        let made = k.r.run("sketch.create", json!({"plane": "XY", "offset": floor - MAGNET_DEPTH, "name": "Magnet circles"}))?;
        for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
            k.r.run(
                "sketch.circle.center",
                json!({"center": [cell[0] + sx * MAGNET_OFFSET, cell[1] + sy * MAGNET_OFFSET], "radius": MAGNET_D / 2.0}),
            )?;
        }
        k.r.run("sketch.finish", json!({}))?;
        let circles = made.get("sketch").cloned().unwrap_or(Value::Null);
        k.extrude(&circles, "Magnet holes", 0.0, MAGNET_DEPTH, 0.0, "cut", None)?;
    }

    k.repeat("Cell row", [1, 0, 0], p.width, None)?;
    k.repeat("Cell rows", [0, 1, 0], p.depth, None)?;

    // Round the four outer corners: keep what lies inside the plate's outline.
    let outline = k.outline("Baseplate outline", 0.0, [0.0, 0.0], [l / 2.0, w / 2.0], PLATE_R)?;
    k.extrude(&outline, "Corners", 0.0, height + 1.0, 0.0, "intersect", None)?;

    Ok(json!({
        "body": plate,
        "grid": [p.width, p.depth],
        "size_mm": [l, w, height],
        "magnet_holes": if p.magnets { p.width * p.depth * 4 } else { 0 },
    }))
}
