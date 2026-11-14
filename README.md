# SolidCraft Gridfinity

A [Gridfinity](https://gridfinity.xyz) bin and baseplate generator for [SolidCraft](https://github.com/tangency-labs/solidcraft). Type a grid size, tick the options, and an editable solid lands in your design: sketches, extrudes and a pattern on the timeline, not a mesh.

A 30 second demo is in [demo/gridfinity-demo.mp4](demo/gridfinity-demo.mp4).

It runs sandboxed as a WebAssembly plugin (`craft:plugin@0.1.0`, capability `commands@0.1`). It needs no files, no network and no clock.

## Install

You need SolidCraft 0.41 or newer.

1. Download `solidcraft-gridfinity-<version>.zip` from the [latest release](../../releases/latest).
2. In SolidCraft, open **Plugins > Install plugin…** (or Preferences > Plugins), pick the zip and choose Install.

The **Plugins > Gridfinity** menu then has two entries.

## Gridfinity bin…

| Parameter | Default | Meaning |
|---|---|---|
| Width | 2 | Grid units along X, 42 mm each (1 to 6) |
| Depth | 1 | Grid units along Y, 42 mm each (1 to 6; at most 25 cells in all) |
| Height | 3 | Height units of 7 mm, without the stacking lip (2 to 20) |
| Wall thickness | 1.2 mm | Walls and dividers (0.8 to 2.4 mm) |
| Stacking lip | on | A lip on the rim that the feet of another bin sit in |
| Magnet holes | on | Four pockets under every cell, 13 mm from its centre |
| Magnet hole diameter / depth | 6.5 / 2.4 mm | The pocket size: the defaults fit 6 x 2 mm magnets (3 to 8.5 mm across, 1 to 4 mm deep) |
| Compartments along X / Y | 1 / 1 | Evenly spaced dividers (1 to 8 each way) |
| Scoop | off | A curved ramp along the front wall, so small parts slide out |
| Label tab | off | A shelf along the back wall with a 45 degree underside |

The bin is centred on the origin and stands on Z 0, the way it prints.

## Gridfinity baseplate…

Width and depth in grid units (1 to 6 each). A plain frame of pockets, 4.65 mm tall.

| Option | Default | Meaning |
|---|---|---|
| Magnet holes | off | A solid floor under the pockets, with four magnet pockets in every cell, 13 mm from its centre: right under the magnets of a bin. The floor is the pocket depth plus 0.6 mm, so with the default pockets it is 3 mm and the plate 7.65 mm tall. |
| Magnet hole diameter / depth | 6.5 / 2.4 mm | As for bins |

## What it builds

- Outer size `42 n - 0.5` mm with 3.75 mm corners; feet with the 0.8 / 1.8 / 2.15 mm profile; a 7 mm floor.
- The stacking lip has the 0.7 / 1.8 / 1.9 mm profile and a 45 degree support under it. Its top keeps a 0.25 mm flat instead of a knife edge, so it is 4.15 mm tall rather than 4.4.
- Dividers, the label tab and the scoop stop 1 mm under the lip, clear of the feet of a bin stacked on top.
- Baseplate pockets have the 0.7 / 1.8 / 2.15 mm profile, with a 0.2 mm flat on each ridge instead of a knife edge.

Not built yet: screw holes, weighted baseplates, half-pitch grids, and different divider heights.

Every tapered surface is a 45 degree extrude of a rounded rectangle, so each feature can be opened and edited afterwards. The sizes are plain numbers: to change the grid, run the plugin again.

## Size limits

SolidCraft gives a plugin 30 seconds per run. Joining many feet onto a body is the slow part, so a bin is limited to 25 cells and a baseplate to 6 x 6. On a 2024 desktop a 2 x 1 bin takes about 3 seconds, a 5 x 5 bin about 12 and a 6 x 6 baseplate about 11.

## Build

You need Rust with the `wasm32-unknown-unknown` target, [`wasm-tools`](https://github.com/bytecodealliance/wasm-tools) and Python 3.

```
./build.sh
```

leaves the package in `dist/solidcraft-gridfinity/` and `dist/solidcraft-gridfinity-<version>.zip`. On Windows, set `CARGO_TARGET_DIR` to a short path if the linker cannot open its output file. The WIT in `wit/` is a copy of SolidCraft's `craft:plugin@0.1.0`.

## Test

```
SOLIDCRAFT_CLI=/path/to/solidcraft-cli python tests/cli_test.py
```

runs the built plugin through `solidcraft-cli` and measures each result: one closed solid, the bounding box, and the volume against a number worked out from the Gridfinity dimensions (they agree to better than 0.02%). It also checks the magnet pocket positions, that a saved design reopens, that two runs in one design do not disturb each other, and that bad inputs are refused naming the field.

## Credits

Gridfinity is by [Zack Freedman](https://www.youtube.com/watch?v=ra_9zU-mnl8), published under the MIT licence. This plugin is an independent implementation of its dimensions and is not affiliated with him or with SolidCraft.

## Licence

MIT, see [LICENSE](LICENSE).
