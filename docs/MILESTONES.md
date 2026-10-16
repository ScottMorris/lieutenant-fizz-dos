# Milestones

The port was built in stages, each in its own folder; the first stages' code now lives in `attic/`. This page records what each stage established; `ARCHITECTURE.md` describes the result and `STATUS.md` what is left.

## Milestone 0: does the sim survive 32-bit x87 floating point?

`m0` runs the real Episode 1 sim through the same C-ABI the web shell uses (`game_new`, `enter_level`, `step`, `state_get`): 16 levels, 3000 ticks each, one scripted input stream, and prints a hash of the 26 HUD state slots every 60 ticks (800 lines). `m0/hash.ts` does the same through the web build's `sim.wasm`.

| Build | Lines differing from the web build's wasm |
|---|---|
| x86_64 Linux (SSE2) | 0 of 800 |
| i586 Linux (x87, default 64-bit mantissa) | 466 of 800, first divergence level 0 tick 120 |
| i586 Linux, 53-bit precision control (`fldcw 0x027F`) | 0 of 800 |
| i586, 53-bit, `opt-level = 3` instead of `"s"` | 0 of 800 |

The whole `lf-sim` (35) and `lf-episode-1` (205, including the playtest bot that clears levels) test suites also pass on i586, with or without the 53-bit shim (`tools/fpu53.c`, an `LD_PRELOAD` constructor).

So: x87 with the precision control set to 53 bits reproduces the web build's f64 simulation exactly here; left at the 64-bit default it drifts within two seconds. A DOS start-up must run `fldcw` before the first tick.

## Milestone 1: Rust in a DOS .EXE

`attic/m1/build.sh` builds a `no_std` Rust library for a custom COFF target (`attic/m1/rust/dos.json`: `i486`, x87 only, `_`-prefixed names), links it with a C `main` using DJGPP, and runs the `.EXE` in headless DOSBox Staging. It prints `Hello from Rust in DOSBox` and `rust_hello returned 3.250000` (an f64 returned on the x87 stack through the C ABI).

Two things the link needed, both in `tools/coff2go32.py`:
- DJGPP's `ld` reads only `coff-go32`, and Rust emits PE-style COFF for a `*-windows-gnu`-like target. The tool adds `-(offset + 4)` to the in-place addend of every `REL32` relocation (go32's convention) and folds the PE sections (`.text$x`, `.rdata`, ...) into go32's `.text`, `.data` and `.bss`. Without the section fold, the code is placed past the loaded image and the program dies silently; without the addend fix, calls land 10 bytes into `puts`.
- `compiler_builtins` objects are dropped from the archive (their weak externals are not understood by coff-go32), so DJGPP's libgcc and libc supply those helpers.

Setup: `tools/fetch-tools.sh` downloads DJGPP (v3.4, gcc 12.2.0), DOSBox Staging 0.83.0 and CWSDPMI into `vendor/` (checksummed, not committed); `rustup component add rust-src --toolchain 1.98.0`; building uses `RUSTC_BOOTSTRAP=1` for `-Zbuild-std` and `-Zjson-target-spec` on the pinned stable toolchain. `tools/dosrun.sh` runs any `.EXE` headless and prints what it wrote to stdout (a crash dump goes to stderr and is not captured).

## Milestone 2: the real sim as a DOS .EXE

`m2/build.sh` builds Episode 1's simulation for the DOS target and runs the same 16 levels × 3000 ticks in headless DOSBox, printing `PASS: 800 checkpoints identical to the web build's wasm`. At the default x87 precision it fails from level 0, tick 120; `fizz_m0(ticks, 1)` sets the 53-bit precision control first.

`m2/sim` and `m2/game` are patched copies of `crates/sim` and `episodes/episode-1/game` (the main repository is untouched). The whole change:
- `#![no_std]` with `extern crate alloc` in both crate roots, plus `use alloc::{boxed::Box, vec, vec::Vec}` at the top of each module.
- `lf_sim::fmath`: two traits (`FloatMath`, `FloatMath32`) that give `floor`, `ceil`, `sqrt`, `sin`, `cos`, `exp`, `hypot`, `atan2`, `powi`, `fract` and `sin_cos` method syntax over the `libm` crate, so call sites needed no edits beyond the `use`.
- `std::ptr` and `std::f64::consts` to `core::`.

`m2/dos-rt` is the DOS side: a global allocator over DJGPP's `malloc`, a panic handler, the 53-bit `fldcw`, and the hash run. The linked `.EXE` is 418 KB unpacked (the web build's wasm is 181 KB).

The run used `libm` on DOS and Rust's own float functions on the web build, and they agree over this input; the web build would have to switch to `libm` too for that to be guaranteed.

## Milestone 3: a VGA mode 13h screen

`m3/export-sprites.ts` writes the 253 sprites the simulation draws, in its own id order, as 8-bit indexed images (`m3/assets/sprites.bin`, 88.8 KB raw). The Rust side fills the simulation's sprite table with those sizes, so the simulation builds the same instance buffer the web renderer draws, and the DOS side blits it into a 320 × 200 frame that `m4/c/vga.c` copies to `0xA0000` in mode 13h after the retrace. The view is 20 × 12.5 tiles; the web build shows about 13 tall.

## Milestone 4: lighting by palette

A VGA palette has 256 entries, so the port uses them as 16 EGA colours × 14 brightness levels (`byte = level * 16 + colour`; slot 255 is the level's clear colour). While sprites are drawn the high nibble is a marker (lit, actor, glowing); a last pass turns it into the brightness of the 8 × 8-pixel cell the pixel is in. The cell's light is the simulation's ambient light, its sun and its point lights worked out as the web shader does for a sprite facing the screen, with the three colour channels folded into one brightness (the palette can only dim a colour, not tint it). Brightness is kept in quarter-levels and a 2 × 2 ordered dither picks the level above or below per pixel, so the 14 levels show no bands. Glowing sprites skip the lighting and characters never go below about 0.62, as in the web shader. See `ARCHITECTURE.md` → The frame.

## Milestone 5: the playable game

`m4/` holds the playable build: keyboard and 60 Hz timing, the shell (title, menus, saves, HUD, cards, dialogue), AdLib and PC-speaker sound, and the start-up system check. `m5/bench` counts instructions per frame natively for speed work. `STATUS.md` lists what plays, what is missing and what has been measured.
