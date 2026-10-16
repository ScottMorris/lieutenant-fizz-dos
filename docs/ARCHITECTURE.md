# Architecture

The DOS port is the main game's Rust simulation with a different front end. This document describes the layers, how a build is made, and the rules that keep the simulation identical to the web build's.

## The layers

```
liminal-hq/lieutenant-fizz            the main game: crates/sim and episodes/episode-1/game (std Rust)
        │  patched copy: no_std, fmath traits over libm
        ▼
m2/sim, m2/game                       the simulation, buildable for a bare i486 target
        │  raw C ABI: fizz_start, fizz_tick, fizz_build, fizz_paint, ...
        ▼
m2/dos-rt                             the DOS runtime (a static library)
   video.rs     sprite blitter, light grid, shade pass, palette markers
   shell.rs     title, pause, options, load/save lists, HUD, cards, dialogue, fades
   ui.rs        pixel font, panels, dimming, the two-tone logo lettering
   save.rs      the 48-byte save format
   preview.rs   the start-up system-check screen
   ui_data.rs   font glyphs and game strings, generated from the main game
        │  coff2go32.py: PE-COFF → go32 COFF
        ▼
m4/c                                  the C layer, built with DJGPP
   main.c       start-up, the main loop, options file, frame-rate and detail policy
   vga.c        mode 13h, palette, retrace, copy to 0xA0000
   kbd.c        IRQ 1 handler, scancode state, press latches
   sound.c      IRQ 0 handler at 560 Hz: OPL2 music, OPL2 and PC-speaker effects
   compat.c     the system check's probes (CPU, FPU, video, sound hardware)
   save.c       files in the current directory
        ▼
FIZZ.EXE  (about 0.9 MB, a 32-bit protected-mode program; CWSDPMI.EXE is its DOS extender)
```

The Rust side owns the game: simulation, drawing, screens, menus, saves and the HUD. The C side owns the machine: the screen, the keyboard, the timer, the sound chips and the files. The two meet at a handful of `extern "C"` functions in each direction (`fizz_*` into Rust; `snd_*`, `save_*`, `rate_*`, `detail_*` and `settings_save` into C).

## Building

`m2/build.sh` builds `m2/dos-rt` for the custom target `m2/dos.json` (an `i486`, x87-only, `_`-prefixed COFF target) with `RUSTC_BOOTSTRAP=1` and `-Zbuild-std`, converts the result, links a hash-run program and compares it with the web build (see below). `m4/build.sh` does the same conversion and links the playable `FIZZ.EXE`.

DJGPP's linker reads only `coff-go32`, and Rust emits PE-style COFF. `tools/coff2go32.py` adds `-(offset + 4)` to the in-place addend of every `REL32` relocation (go32's convention), folds the PE sections (`.text$x`, `.rdata`, …) into go32's `.text`, `.data` and `.bss`, and drops the `compiler_builtins` objects (DJGPP's libgcc and libc supply those helpers). `m4/c/chkstk.c` provides the `_alloca` helper the stack probes would need.

`m3/export-sprites.ts`, `m4/export-sound.ts` and `m4/export-ui.ts` read the main game and write `m3/assets/sprites.bin` (253 sprites as 8-bit indexed images), `m4/assets/sound.bin` (the converted music and effects) and `m2/dos-rt/src/ui_data.rs` (the pixel font and the strings). They are committed, so a build needs neither Bun nor the main game.

## Keeping the simulation identical

The web build runs the simulation in `f64` on WebAssembly. A DOS build uses the x87, whose default 64-bit mantissa gives different results. Two rules keep them equal:

- The x87 precision control is set to 53 bits (`fldcw 0x027F`) before the first tick.
- Maths functions come from the `libm` crate through the `fmath` traits in `m2/sim`, not from the C library.

`m2/build.sh` is the check: 16 levels × 3000 ticks on one scripted input stream, a hash of the 26 state slots every 60 ticks, 800 lines compared with the output of the web build's wasm. At the default precision the first difference is at level 0, tick 120. Anything that changes the simulation or the order of its floating-point operations must keep this at `PASS`.

## The frame

A 320 × 200 byte buffer, shown in VGA mode 13h. The palette is 16 EGA colours at 14 brightness levels: `byte = level * 16 + colour`, level 1 black to level 14 full, level 0 the colour at full brightness for the markers, and slot 255 the level's clear colour.

1. `fizz_build` asks the simulation for the draw list (an instance buffer, painter's order).
2. `fizz_paint` works out the light on each 8 × 8 cell from the sim's ambient light, sun and point lights (a single brightness per cell, in quarter-levels), clears the frame, and blits each sprite with fixed-point steps (copy, flip, scale; a rotated sprite maps each pixel back through its matrix). A sprite's high nibble is a marker: lit, actor (never below level 9) or glowing (full brightness).
3. Where the light is the same everywhere (the map and the day levels) the markers are the final levels and nothing more is needed. Otherwise a shade pass turns each marker into its cell's level, four pixels at a time, with a 2 × 2 ordered dither between neighbouring quarter-levels so 14 levels show no bands.
4. The shell draws its screens, HUD and text over the finished frame; menus dim what is behind them by lowering the level nibble.
5. The C layer waits for the vertical retrace and copies the frame to video memory.

The camera is snapped to whole pixels so that things at rest stay at rest relative to each other as it glides.

## Time, input and sound

The simulation steps at 60 Hz from a clock kept by the sound handler (the BIOS clock when sound is off); the screen draws as often as it can between ticks, or on every second tick at 30 Hz. A stall of more than six ticks resets the clock rather than rushing to catch up.

`kbd.c` replaces the IRQ 1 handler, tracks held keys and latches each press so a tap shorter than a tick is not lost. Input bits go to the shell, which passes the game's bits to the simulation; a key a menu consumed is held back from the simulation until it is released.

`sound.c` replaces the IRQ 0 handler and runs it at 560 Hz. It plays IMF-style OPL2 music (rhythm-mode drums; channel 5 is kept for effects) and effects on the OPL2 or the PC speaker. The sound choice is AdLib for everything, the music on AdLib with effects on the speaker, the speaker alone, or off.

## Files

- `AUTO.SAV`, `SLOT1.SAV` to `SLOT4.SAV`: 48-byte `FZSV` saves (lives, score, next extra life, Fizz, levels cleared, map position, time played).
- `FIZZ.CFG`: 12 bytes, `FZC`, a version, then music, effects, the effects' device, frame rate, detail, night, the sound choice and a spare. It is written after each change on the Options screen and when the start-up screen is left.
- File names are 8.3; a longer name fails on DOS.

## Measuring

`m5/bench` builds the drawing and shell code natively as a 32-bit x87 program and counts user-space instructions per stage with `perf_event_open`, then prints a frame checksum for five scenes. Speed work starts there, then moves to DOSBox with `cycles=fixed 26800` (a 486DX2-66) for milliseconds per frame. The bench's stub of the C functions lets the shell run without a DOS machine, which also makes menu behaviour testable (`PAUSETEST=1`).
