# Performance

Where a frame's time goes on a 486, what has been done about it, and what could be done next. The figures come from three places, and they disagree in an important way:

- **`m5/bench`** counts the instructions the drawing code and the sim spend on a frame, natively as a 32-bit x87 program. Where the CPU's counter cannot be opened (a virtual machine), `m5/bench/profile.sh` counts them under callgrind instead.
- **DOSBox Staging** at `core = normal` and `cycles = fixed 26800` (a 486DX2-66 by the usual rule) gives milliseconds per frame from the game's own `per frame:` report (`FIZZ.EXE <level> 300 nosync nosound`). `tools/dosbox.conf` runs at `cycles = max`; the timing runs use a copy with those two lines changed.
- **`m5/bench/cyc486.py`** weights each executed instruction by its i486 clock count (`profile.sh --cycles`). DOSBox charges about one cycle for any instruction, so it hides the cost of the x87 on a real chip: a `fistp` is about 31 clocks on a 486, an `fdiv` 73 and an `fsqrt` 85. The estimate assumes every access hits the cache and no wait states, so a real 486 is slower still; it is for comparing code, not for promising a frame rate.

## Where the time goes

Per frame, a sim step plus the build and the paint, before the changes below (bench, 300 frames):

| Scene | Instructions | Est. 486 cycles | x87 share of cycles |
|---|---|---|---|
| Overworld map | 1.51 M | 3.75 M | 51% |
| Day level (0) | 0.97 M | 2.78 M | 56% |
| Crystal Caves (1) | 1.92 M | 4.32 M | 59% |
| Level 9, a result card up | 2.69 M | 5.04 M | 42% |

- **The sim's step is cheap.** 15,000 to 28,000 instructions a tick, about 50,000 to 65,000 486 cycles: roughly 1 ms on a DX2-66, about 5% of the machine at 60 ticks a second.
- **The paint was most of the frame**, and within it: the backdrop (one tinted tile repeated about 380 times, 98,000 pixels on a 64,000-pixel screen) through a colour table a pixel at a time; about 400 instructions of float decoding per sprite (each `f32 as i32` is a control-word swap, a `fistp` and a range check); and the lighting grid, which took a square root and two divisions per lit cell per light. The lighting was only about 200,000 instructions, but about 1.0 to 1.4 M 486 cycles: the dearest thing in a frame on a real chip, and nearly invisible in DOSBox.
- **The sim's draw list (`World::render` and `InstanceBuffer::push`)** costs 110,000 to 280,000 instructions, 4 to 12 ms in DOSBox, and up to 0.8 M 486 cycles on the map, most of it f64 work in the main game's code.
- **Paused screens drew everything every frame.** A result card, the pause menu or a dialogue repainted the whole lit scene and then dimmed it, 1.5 M instructions for a picture that does not change.

## What was changed

All in `m2/dos-rt` (the DOS runtime); nothing in the simulation or its copies. The frame checksums are unchanged for every change except the integer lighting.

- **Baked tiles.** An opaque tile that needs its colours changed (a tint, or the light marker of an evenly lit level) is worked out once into a per-sprite slot and then copied a row at a time, a 16-byte row as four word moves. The slot is kept between frames and redone only when the tint or marker changes. The cave's backdrop goes from a table lookup a pixel to a copy.
- **Sprite headers read once.** The sheet's headers are parsed into a table on first use instead of from bytes on every sprite.
- **Integer sprite placement.** A sprite at its own size (nearly all) is placed from the bits of its `f32` position: `floor_scaled` gives `floor(x * 32)` exactly with integer shifts, and the rounding is done on that. The flags, the sprite index and the tint are read from their bits too. Scaled and turned sprites keep the float path. The results are bit for bit the float path's.
- **Integer lighting.** Light positions are in quarter pixels and the falloff `(1 - u)^2 / u` comes from a 1,024-step table over the squared distance, so a lit cell is a few integer steps. The light differs from the float version by one level on at most 0.13% of pixels, with no colour change and no bias up or down; the float version itself was not stable (adding a counter to it changed the day level's checksum through x87 register spills). `m5/bench/expected-checksums.txt` was regenerated for the cave, night cave and level 9 scenes.
- **Kept scene.** When the sim has not stepped since the last frame and its output, its lights, the draw list, night and detail are all bit for bit the same, the shaded scene from the last frame is copied back and only the interface is drawn over it.
- **Interface.** `dim` works on four pixels at a time with the same result as its table (checked exhaustively for every byte and step), and glyphs that are wholly on the screen are drawn without clipping each pixel.

## Results

Bench, instructions a frame (sim step, build and paint):

| Scene | Before | After | Change |
|---|---|---|---|
| Overworld map | 1.51 M | 1.16 M | −23% |
| Day level | 0.97 M | 0.72 M | −26% |
| Crystal Caves | 1.92 M | 1.15 M | −40% |
| Crystal Caves, night | 2.02 M | 1.24 M | −39% |
| Level 9, a result card up | 2.69 M | 1.11 M | −59% |

Estimated 486 cycles a frame: map 3.75 M → 2.62 M, day 2.78 M → 1.58 M, caves 4.32 M → 2.43 M, level 9 with its card 5.04 M → 1.82 M.

DOSBox at `cycles = fixed 26800`, the real DOS build, milliseconds a frame (300 ticks of the scripted walk):

| Scene | Build | Paint before | Paint after | Frame before | Frame after |
|---|---|---|---|---|---|
| Day level | 4.4 | 31.9 | 23.0 | 37.5 (27 fps) | 28.6 (35 fps) |
| Overworld map | 11.9 | 42.1 | 30.8 | 55.1 (18 fps) | 43.8 (23 fps) |
| Crystal Caves, detail high | 7.0 | 64.9 | 35.5 | 73.2 (14 fps) | 43.8 (23 fps) |
| Crystal Caves, detail low | 7.0 | 36.8 | 26.2 | 45.1 (22 fps) | 34.5 (29 fps) |
| Crystal Caves at night | 7.0 | 68.3 | 38.8 | 76.6 (13 fps) | 47.1 (21 fps) |
| Level 9, detail high | 7.8 | 75.1 | 39.2 | 83.9 (12 fps) | 48.0 (21 fps) |
| Level 9, detail low | 7.8 | 56 | 31.1 | 65 (15 fps) | 39.9 (25 fps) |

A frame here is the build, the paint, the copy to the screen (0.6 ms) and one sim step (0.4 to 0.7 ms). The level 9 run in DOSBox is the level itself, not the result card the bench ends on.

## Would a fixed-point simulation help?

Not enough to be worth what it costs.

- **What it would save.** The sim's step is about 5% of a 486DX2-66 at 60 ticks a second. Its x87 work is 40 to 70% of the step's estimated cycles, so a perfect fixed-point rewrite would give back about 2 to 4% of the machine. The heavy float work is in drawing, and all of that is the DOS port's own code, where it can be removed without touching the game; most of it now has been.
- **What it would cost.** The port runs the main game's simulation unchanged, and `m2/build.sh` proves it: 800 state hashes over 16 levels are identical to the web build's wasm. Fixed point rounds differently from f64 from the first tick (the x87 at its default 64-bit precision already diverges at tick 120), so jumps, collisions and enemy paths would drift from the web build, saved levels would play differently, and every rule change in the main game would have to be ported by hand into a second, different simulation. The determinism check would have to go.
- **Where fixed point does pay** is everything between the sim and the screen, which is not bound to the web build: the sprite placement and the lighting above, and the next items below.

## Next opportunities

Roughly in order of what they would save on a real 486.

1. **The sim's draw list.** `World::render` and `InstanceBuffer::push` are 0.2 to 0.8 M 486 cycles a frame (4 to 12 ms in DOSBox): every instance is converted from f64 to f32 and packed into a 4×4 matrix for a GPU, then unpacked again here. The map recomputes its shores, lips and path edges from the tile map every frame. This is the main game's code (`m2/sim`, `m2/game`), so it is best changed there: an integer side list (sprite, tint, flags, pixel position) written alongside the matrices, or a cached map layer, would help the web build too. Skipping the build while the sim is paused would save another 0.1 to 0.2 M instructions on paused screens.
2. **Fuse the shade pass into the copy to the screen.** Shading reads and writes the whole frame (about 230,000 instructions in the caves) and the copy to `0xA0000` reads it again; one pass that shades into video memory saves a full pass over 64 KB. On real hardware the VGA write is the slow part (an ISA card can take 5 to 10 ms for 64,000 bytes), so this matters more there than in DOSBox.
3. **Backdrop overdraw.** The cave's backdrop is drawn under opaque tiles that hide about a third of it. Opaque tiles sit on the world grid, so a per-row coverage mask built from them would let the backdrop skip hidden spans.
4. **Full-screen dims and fades through the palette.** A fade or the dim behind a menu is a remap of the 256 palette slots, so it could be 768 writes to the VGA's DAC instead of 64,000 pixel lookups. The frame buffer (and screenshots) would then hold the undimmed picture.
5. **Turned sprites** divide twice per pixel; stepping through the sprite with fixed-point increments, as the scaled path does, would remove the divisions.
6. **Only copy what changed while paused.** With the kept scene, a paused frame differs from the last only where the menu animates; copying only the changed rows to video memory would make paused screens nearly free on slow VGA cards.
7. **Not worth it:** `opt-level = 3` for the whole build gains 1 to 6% for a larger binary. Mode X hardware scrolling, the classic answer, does not fit the half-speed parallax backdrop and the palette lighting, which change every pixel when the camera moves.

## What was run

- `tools/check.sh`: licence headers, the simulation copies, the determinism run against the web build (`PASS: 800 checkpoints`), the DOS build, the key map, the frame checksums and the pause-menu test.
- The bench and callgrind figures above, and DOSBox Staging 0.83 at `cycles = fixed 26800` on the scripted runs; a headless screenshot of the cave at night.
- Not run: real hardware, and any listening test.
