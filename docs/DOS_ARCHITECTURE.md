# DOS runtime architecture

How `FIZZ.EXE` runs on the machine: the process and memory model, start-up and shutdown, the two interrupt handlers, the main loop's timing, the VGA and sound hardware, the files, the C/Rust boundary and the browser build. [ARCHITECTURE.md](ARCHITECTURE.md) is the short overview (the layers, the build, the determinism rules and the frame pipeline) and is not repeated here; [RUST_ON_DOS.md](RUST_ON_DOS.md) covers the toolchain (the target spec, `-Zbuild-std`, `tools/coff2go32.py` and linking). [PERFORMANCE.md](PERFORMANCE.md) has the frame costs.

## Process and memory model

`FIZZ.EXE` is a DJGPP v2 program: a real-mode stub followed by a 32-bit go32 COFF image. The stub loads a DPMI host — `CWSDPMI.EXE`, shipped beside the game (`dist/fizz-dos-demo`, `tools/make-site.sh`), unless one is already present (DOSBox, Windows) — switches to protected mode and runs the image with 32-bit flat code and data selectors. Everything runs on one thread; the only concurrency is the two interrupt handlers.

**Stripping.** `m4/build.sh` writes `m4/c/fizz.exe` with its symbol table (6,984 symbols and their names, about 207 KB), DWARF sections (about 55 KB) and a compiler `.comment` section, which the checks and `tools/dosrun.sh` run as they are. What a release and the browser bundle carry is a stripped copy (`tools/strip-exe.sh`, DJGPP's `strip --strip-all`): 1,123,051 bytes become 853,504, 269,547 fewer (24%). The script refuses a copy unless the 2,048-byte go32 stub, the entry point (`0x19f0`) and the bytes of `.text` (335,896) and `.data` (515,072) are exactly the original's, `.bss` is the same size and no symbols are left; the loader needs only the section table, which `strip` rewrites (the code moves 4 KB earlier in the file). Run in DOSBox, the stripped and unstripped programs gave identical frames and palettes for nine screens (a level, another level, the map, the title, the opening titles at two ticks, the ending, a cinematic scene and Options) and identical sound-driver output (the title song and the opening titles' song on the OPL2, and the speaker). DJGPP prints a crash's traceback as addresses, and `symify` turns them into names using the symbols in the program, so a stripped program cannot be read; the release's `-debug.zip` holds the unstripped `FIZZ.EXE` for that.

| Region | What lives there | Source |
|---|---|---|
| `.text` | C code, the Rust runtime and simulation | `m4/c/*.c`, `m2/dos_rt_go32.a` |
| `.data` | initialised data, including Rust's `.rdata`: the sprite sheet (`m3/assets/sprites.bin`, about 87 KB, `include_bytes!` in `m2/dos-rt/src/video.rs`) the sound data (`m4/assets/sound.bin`, about 279 KB, `include_bytes!` in `m2/dos-rt/src/sound_data.rs`) and the opening titles' data (`m4/assets/titles.bin`, about 54 KB, `include_bytes!` in `m2/dos-rt/src/titles.rs`) | `tools/coff2go32.py` folds `.rdata`, `.xdata`, `.pdata` and the like into `.data` |
| `.bss` | zero-initialised statics: the C frame buffer (`static unsigned char frame[64000]`, `m4/c/main.c`), the kept scene `STILL` (a 64,000-byte frame, 2,048 × 20 instance words and a 146-word header, about 228 KB), the bake cache `BAKES` (256 slots of 16 × 16 bytes plus a `u64` key each, about 66 KB), the sprite table, the falloff and shade tables, the key arrays | `m2/dos-rt/src/video.rs`, `m4/c/kbd.c` |
| heap | everything the simulation allocates (`alloc`: vectors, boxed state) | DJGPP `malloc` |
| stack | DJGPP's default stack; neither `_stklen` nor `stubedit` changes it | — |

The two big caches are deliberately all-zero at start (`BakeCache`'s keys use bit 40 to mark a filled slot so that zero means empty), so they sit in `.bss` and take no room in the 0.9 MB EXE.

**Heap.** `m2/dos-rt/src/lib.rs` installs a `#[global_allocator]` that forwards `alloc`, `dealloc` and `realloc` to DJGPP's `malloc`, `free` and `realloc`. DJGPP's blocks are 8-byte aligned, which covers every type the simulation uses (a `debug_assert!` checks the alignment). DJGPP grows the heap with `sbrk` from DPMI memory, so the heap is not limited to conventional memory. The panic handler prints `PANIC` and the line through `printf`, flushes, and spins.

**Reaching video memory.** Protected-mode code cannot address `0xA0000` through its own data selector by default. `vga_open` (`m4/c/vga.c`) calls `__djgpp_nearptr_enable()`, which raises the limit of the program's data segment to 4 GB; `__djgpp_conventional_base` is then the offset that turns a linear address into a near pointer, so video memory is `__djgpp_conventional_base + 0xA0000`. The base can change when the heap grows, so `vga_present` reads it on every frame rather than caching a pointer. Near pointers turn off DJGPP's memory protection (a stray pointer is no longer caught by a segment limit); `vga_close` calls `__djgpp_nearptr_disable()` after going back to text mode. If near pointers are refused (some DPMI hosts do not allow them), `FIZZ.EXE` prints `no near pointers` and exits.

**Memory locking.** A DPMI host may page memory out. An interrupt handler that touches a page that is not present would fault inside the interrupt, so both handlers lock what they use with `_go32_dpmi_lock_code` and `_go32_dpmi_lock_data` before hooking the vector (see [Interrupts](#interrupts)). Lock ranges for code are taken from a function's address to a following empty marker function (`isr`…`isr_end`, `opl_write`…`timer_end`); `m4/build.sh` compiles with `-fno-toplevel-reorder` so GCC keeps the functions in source order and the ranges cover what is between them.

## Start-up and shutdown

`main` in `m4/c/main.c`, interactive path (no tick count on the command line):

```
settings_load()                 FIZZ.CFG → music, effects, device, frame rate, detail, night
parse argv[3..]                 overrides for this run (nosound, adlib, mixed, speaker, fps30, lowdetail, night, ...)
compat_probe()                  the system check's hardware probes, before the game takes its memory
fizz_start(level, 1)            only when the machine has what the game needs (fizz_preview_ready)
                                Rust: fldcw 0x027F (53-bit precision), new game, title/map/level, sprite size table
fizz_night(night)
no VGA?                         the check as text (fizz_preview_text), wait for Esc, exit
kbd_install()                   IRQ 1 handler (interrupt 9)
vga_open()                      near pointers on, INT 10h AX=0013h
palette slots 0..254            from the 16 EGA colours × 15 levels (slot 255 is set every frame)
speed_test()                    3 + 30 trial frames timed with uclock(), when the game can start
system-check loop               fizz_preview → vsync → present; Enter/Space go on when the game can start, Esc leaves
wait for Enter/Space release    and clear their latches, so the title menu does not take the same press
snd_init(sound.bin, mode)       detect the OPL2, lock memory, IRQ 0 handler (interrupt 8), PIT to 560 Hz
snd_set_music/fx/fx_device      from FIZZ.CFG
main loop                       until fizz_quit()
vga_close()                     INT 10h AX=0003h, near pointers off
kbd_remove()                    old interrupt 9 vector back
snd_shutdown()                  old interrupt 8 vector back, PIT to 18.2 Hz, OPL2 and speaker silent
```

- The level argument is a number, `map` (254 to the game) or nothing (255, the title screen). A second argument (ticks) selects a scripted run: no keyboard hook, no system check, one tick a frame (`fizz_scripted` supplies the input), and with `shot` the frame and palette are written to `FRAME.RAW` and `PAL.RAW` at the end. `bench`, `soundtest`, `savecfg`, `preview` and the screen names (`pause`, `options`, `load`, `save`, `dialogue`, `loadslotN`) are check modes listed in the comment at the top of `main.c`.
- The precision control is set inside `fizz_start`, before the simulation's first tick (`fpu_53` in `m2/dos-rt/src/lib.rs`); see [ARCHITECTURE.md](ARCHITECTURE.md#keeping-the-simulation-identical) for why.
- **The system check** runs before the sound handler takes the timer, so `uclock()` (which reads the PIT and the BIOS tick count) is still trustworthy, and before `fizz_start`, so the free memory it reads is what the game can have. `compat_probe` (`m4/c/compat.c`) classifies the video adapter with `compat_video`: the display combination (INT 10h AX=1A00h: BL 8 is a colour VGA, 7 a monochrome one; the MCGA codes are not taken as a VGA), then EGA information (INT 10h AH=12h BL=10h: BL changes on an EGA), then the BIOS equipment word (INT 11h bits 4-5: 01 or 10 a CGA, 11 an MDA or Hercules). It also asks the mouse driver (INT 33h AX=0), the game port (port `0x201` not `0xFF`), the OPL2 (`snd_detect_opl`), a Sound Blaster DSP reset at `0x226`/`0x22A`/`0x22E` answering `0xAA` (`snd_detect_blaster`), an x87 (after `fninit`, `fnstcw` must read back the default control word's bits), free physical memory (`_go32_dpmi_remaining_physical_memory`) and the processor name (`uname`). `speed_test` renders three warm-up frames, then times 30 frames of `fizz_build` + `fizz_paint` (no present) and reports frames a second. The slots are the `C_*` enum in `m4/c/compat.h` (and the adapter classes its `VIDEO_*` enum), mirrored by the constants in `m2/dos-rt/src/preview.rs`, which draws the screen and grades the speed: 90 or more smooth, 62 or more fine, 40 or more borderline, below that too slow.
- **What the game needs** is a VGA (mode 13h), an x87 and 10 MB of free memory (`MEMORY_NEEDED_KB` in `preview.rs`): starting the game takes about 9.4 MB at once, nearly all of it the simulation's instance buffer (120,000 instances of 20 floats, written whole). When one is missing `fizz_preview_ready` says no, `fizz_start` is not called, the speed test is not run, and the screen's footer names what is missing; only Esc leaves. Mode 13h is set only on a VGA: without one the same rows are written in colour on the text screen (`fizz_preview_text`, bytes below 8 setting the colour) and the program waits for Esc. With `preview` the picture is still drawn into the frame for `FRAME.RAW` and the text goes to standard output, so each case can be checked headless (DOSBox's `machine = ega`, `cga` or `hercules`, or a small `memsize`).
- The system check is read-only: Esc restores text mode and the keyboard and exits before any sound is set up, and Enter or Space go on. The sound mode is auto unless the command line names one; the music, effects and effects' device come from `FIZZ.CFG` (the Options screen).
- **Restoring the machine.** Both hooks also register their remove function with `atexit`, and both remove functions are idempotent (an `installed` flag), so the vectors come back on any normal exit path, including `exit()` from the C library. A crash inside protected mode (a page fault, a panic's endless loop) does not run `atexit` handlers.

## Interrupts

| IRQ | Vector | Handler | Rate | Chains to the old handler | EOI | Locked |
|---|---|---|---|---|---|---|
| 0 (PIT channel 0) | 8 | `timer_isr`, `m4/c/sound.c` | 560 Hz | about 18.2 times a second | the old handler's, or `outportb(0x20, 0x20)` | code `opl_write`…`timer_end`; `music`, `fx`, `tick_count`; the whole sound blob |
| 1 (keyboard) | 9 | `isr`, `m4/c/kbd.c` | per scancode byte | never | `outportb(0x20, 0x20)` | code `isr`…`isr_end`; `keys`, `latch` |

Both are protected-mode handlers installed with DJGPP's `_go32_dpmi_get_protected_mode_interrupt_vector` (saving the old vector), `_go32_dpmi_allocate_iret_wrapper` (a stub that saves the registers, loads the program's selectors, calls the C function and returns with `IRET`) and `_go32_dpmi_set_protected_mode_interrupt_vector`. Removal sets the saved vector back and frees the wrapper. Variables shared with the main program are `volatile`, and neither handler allocates.

The sound handler's smaller statics (`want_music`, `want_fx`, `spk_*`, `old_acc`, `old_isr`, `fx_dev` and the table pointers) are not locked by name; only the ranges in the table are.

### Keyboard (IRQ 1)

`isr` reads one byte from port `0x60` per interrupt. `0xE0` and `0xE1` prefixes are skipped, and the next byte is recorded by its low seven bits, so the grey arrows and the keypad arrows are the same key (`m4/c/keymap.h`). A make code sets `keys[code]` and, if the key was up, sets `latch[code]`; a break code (bit 7) clears `keys[code]`. The latch is set only on the up-to-down edge, so typematic repeats do not add presses. The handler does not chain to the BIOS: while it is installed nothing reaches the BIOS key buffer, and Ctrl+Break and Ctrl+Alt+Del are not seen by DOS.

Reading the state (main program side):

- `kbd_held()` — the game's input bits from what is held now, through `keymap_bits` (`m4/c/keymap.c`): arrows or WASD move, Ctrl or Z jump, Alt or X pogo, Space or C fire, Enter confirm, Esc menu. The bits are the sim's (`IN_*` in `keymap.h`).
- `kbd_tick_bits()` — what is held **or** latched, for the 16 game keys, and clears those latches. A key pressed and released between two ticks has `keys` clear but `latch` set, so the tick still sees it: a tap shorter than a tick (1/60 s) is never lost. It also carries `IN_ANY` (bit 2048) for a key of any kind pressed since the last call: the ISR sets a flag on each new press (never on a repeat, so a key already down does not count), and this clears it. Only the opening titles read it.
- `kbd_pressed(sc)` — one latched press of a key, cleared on read; used by the main loop for its own keys (PgUp/PgDn change level, R restarts, Backspace goes to the map, N night, M music, E effects) and by the system check.
- `kbd_down(sc)` — the held state of one key.

The shell (`m2/dos-rt/src/shell.rs`, `advance`) turns held bits into edges for its menus and holds a key a menu consumed back from the simulation until it is released (`swallow`).

### Timer (IRQ 0)

`snd_init` programs PIT channel 0 in mode 3 (`outportb(0x43, 0x36)`) with divisor 2131 (1,193,182 / 560, so about 559.9 Hz), the IMF clock the music data is written for. Each interrupt:

1. `tick_count++` — the game's clock (`snd_ticks`).
2. `take_requests()` — picks up a song or effect requested by the main program through `want_music` / `want_fx` (a song change silences the chip and starts the new stream; an effect keys off channel 5 and starts its stream, or starts the speaker's divisor list).
3. With an OPL2, steps the music stream and the effect stream once each (`stream_step`).
4. With effects on the speaker, every fourth tick (140 Hz) loads the next PIT channel 2 divisor, or turns the speaker off at the end.
5. Keeps DOS's clock running: an accumulator adds 18,207 per tick and, when it passes 559,900, subtracts that and calls the old handler (`pushfl; lcall *old_isr.pm_offset`), which advances the BIOS tick count and sends the end of interrupt. On every other tick the handler sends `0x20` to port `0x20` itself. The old handler therefore still runs at an average of 18.2 Hz and the DOS time of day stays correct.

`snd_shutdown` restores the vector first, then writes divisor 0 (65,536, the BIOS's 18.2 Hz), frees the wrapper and silences the OPL2 and the speaker.

With the sound off (`nosound`, or the choice Off), IRQ 0 is not hooked, and the game's clock falls back to `uclock()`.

## Timing and the main loop

**The clock.** `clock560()` in `m4/c/main.c` is the game's time in 1/560 s: `snd_ticks()` while the sound handler is installed, otherwise `uclock() * 560 / UCLOCKS_PER_SEC`. `uclock()` is not used for anything else while the handler runs, because the handler has reprogrammed the timer it reads (`stamp()` returns 0 then, so the per-stage timing report is only meaningful with `nosound`).

**One pass of the interactive loop:**

```
elapsed = clock560() - base;  owed = elapsed * 60 / 560           ticks the sim should have run
fizz_quit()?  → leave
main-loop keys (PgUp, PgDn, R, Backspace, N, M, E)
owed > done + 6?  → base = now, done = owed = 0                    stall: start again rather than rush
while owed > done:                                                 fixed 60 Hz steps
    fizz_tick(first ? kbd_tick_bits() : kbd_held())                the first tick also gets quick taps
    done++
choose_music()                                                     title / map / level / boss song
30 Hz and fewer than 2 ticks since the last frame?  → continue
show(alpha = ((elapsed * 60) % 560) / 560):
    fizz_build(alpha)      the sim writes its draw list, interpolated alpha of the way to the next tick
    fizz_paint(frame)      lights, sprites, shade pass, kept scene, shell overlay; returns the clear colour
    vga_palette(255, clear colour)
    vga_vsync()            wait for the start of the vertical retrace
    vga_present(frame)     64,000 bytes to 0xA0000
auto policies (every 32 frames)
```

The loop never sleeps; it spins on `clock560()` until a tick is owed or a frame can be drawn. At 60 Hz a frame is drawn on every pass (each locked to a retrace); `alpha` is the fraction of the current 1/60 s that has gone by, so motion between ticks is interpolated by the simulation's own `render`.

**Frame rate.** The setting is auto, 60 Hz or 30 Hz (`rate_mode`, `rate_set`, `rate_active`). At 30 Hz a frame is drawn only once two ticks have passed since the last one. In auto, the loop sums the cost of `show()` (in 1/560 s, retrace wait included) over 32 frames and compares the average with 1/60 s:

| Average frame cost (percent of 1/60 s) | Rate in use |
|---|---|
| below 67 | 60 Hz |
| 67 to 100 | unchanged |
| over 100, up to 170 | 30 Hz (a steady pace beats an uneven one) |
| over 170, up to 185 | unchanged |
| over 185 | 60 Hz (so slow that a cap would only throw frames away; draw as often as possible) |

**Detail.** The setting is auto, high or low (`detail_mode`, `detail_set`, `detail_active`); low leaves the backdrop layers out of the paint (`fizz_detail`, `set_low_detail` in `video.rs`). Auto detail is evaluated in the same 32-frame window, so only while the frame rate is also auto: it goes low when the average frame costs more than 45 ms and back to high when a low-detail frame costs under 17 ms.

The clock's resolution is 1/560 s (about 1.8 ms), and `uclock()` is finer; a scripted run (`FIZZ.EXE <level> <ticks>`) does one tick a frame regardless of time so that its screenshots repeat.

## Video

**Mode.** `vga_open` sets mode 13h through `__dpmi_int(0x10)` with AX = 0x13 (a real-mode interrupt reflected by the DPMI host): 320 × 200, 256 colours, linear at `0xA0000`. `vga_close` sets mode 3 (80 × 25 text).

**Palette.** `build_palette` in `main.c` fills slots 0 to 239 from the 16 EGA colours (as 6-bit DAC values, in the sprite DSL's order) and `vga_palette` writes them through the DAC: the index to port `0x3C8`, then red, green and blue to `0x3C9`.

| Slots | Byte | Meaning |
|---|---|---|
| 0–15 | `0x0c` | level 0: the colour at full brightness (a marker before the lighting pass) |
| 16–223 | `0xLc`, L = 1–13 | the colour at `(L − 1) / 13` of full, rounded; level 1 is black |
| 224–239 | `0xEc` | level 14: full brightness; the user interface draws here (`ui::FULL`) |
| 240–254 | `0xFc` | unused, black |
| 255 | `0xFF` | the level's clear colour, rewritten before every present from `fizz_paint`'s return value (`0x00RRGGBB`, 6 bits a channel) |

**Frame and present.** The frame is the 64,000-byte `frame` array in `main.c`, passed to Rust as a raw pointer and wrapped as a `&mut [u8]` of `W * H` (`m2/dos-rt/src/lib.rs`). `vga_vsync` polls bit 3 of the input status register (`0x3DA`): it waits for any retrace in progress to end and then for the next to start. `vga_present` is a single `memcpy` to video memory; it is not split around the beam, so where 64 KB takes longer than the retrace (a slow ISA card) the copy can run on into the visible frame.

**Drawing.** All drawing is in `m2/dos-rt`; [ARCHITECTURE.md](ARCHITECTURE.md#the-frame) describes the pipeline. Where the pieces are:

- `m2/dos-rt/src/video.rs` — `build` (the simulation's `render` with culling and night flags), `paint` (the camera snapped to whole pixels, `light_levels` for the 40 × 25 grid of 8 × 8 cells with integer falloff from a table, the clear to slot 255, `blit` and `blit_turned` per instance, the uniform-light shortcut, `shade` four pixels at a time with a 2 × 2 dither), `TintCache`, `BakeCache` and the kept scene `Still`.
- `m2/dos-rt/src/shell.rs` — the screens over the frame (title, pause, options, load and save lists, HUD, cards, dialogue, toasts, caption words) and fades: a fade is 12 ticks of `ui::dim` over the whole screen.
- `m2/dos-rt/src/titles.rs` — the opening titles, which draw every pixel of the frame themselves (so `build` and `paint` skip the sim's scene while they are up): per-tick tracks from `titles.bin`, `fx::Tween` in Q30 for the tweens and Q32 for the drifts, the fade by `ui::dim` and the flash as a white fill.
- `m2/dos-rt/src/ui.rs` — the pixel font (glyphs in the generated `ui_data.rs`), panels, word wrap, and `dim`, which lowers the level nibble of each byte four at a time (slot 255 dims to black, slot 16).
- `m2/dos-rt/src/preview.rs` — the system-check screen.

## Sound

**Data.** `m4/assets/sound.bin` is generated by `m4/export-sound.ts` from the main game's Undertone patterns, converted by the code in `m4/sound` (`fizz-dos/song.ts`, `music.ts` and `opl.ts` for the OPL2 songs, `sfx.ts` for the effects, `patches.ts` for the instruments). It is linked into the EXE with `include_bytes!` (in `sound_data.rs`, which also reads the song table for the screens: the title and extra songs, and a song's length in ticks) and handed to the C driver through `fizz_sound_data` / `fizz_sound_len`. The extras are the story screens' songs, which `music_key` in `shell.rs` picks by position; the opening titles' song is eight bars at 150 bpm, 7168 ticks, and the export fails if it is any other length. Layout (little endian):

```
"FZSN"  u8 version (1)  u8 songs  u8 effects  u8 captions  u8 levels
songs     × { u32 offset, u32 length }                                     an IMF chunk
effects   × { u32 oplOffset, u16 oplLength, u32 speakerOffset, u16 speakerFrames }
captions  × u8 effect (255 none), in the sim's caption order
levels    × u8 song, in the sim's level order
u8 title song, u8 map song, u8 boss song
u8 extras, then that many u8 songs: the ending, the yard, the lab, the launch, deep space, the opening titles
data
```

An IMF chunk is a 16-bit byte length followed by four-byte records: register, value, and a 16-bit wait in 560 Hz ticks before the next record. A speaker effect is a list of 16-bit PIT divisors (1,193,180 / frequency, 0 for silence), one per 140 Hz frame.

**OPL2.** The chip is at ports `0x388` (address and status) and `0x389` (data). `opl_write` writes the register number, reads the status port 6 times, writes the value, then reads the status port 35 times; the reads are the delay the OPL2 needs after an address write (about 3.3 µs) and a data write (about 23 µs), since each ISA port read takes roughly a microsecond. `opl_detect` is the usual timer test: reset both timers and the IRQ flag (register 4 to `0x60`, then `0x80`), read the status (top three bits must be 0), start timer 1 at `0xFF` (register 2, register 4 to `0x21`), wait 3 ms, read the status again (top bits must be `0xC0`), and reset the timers. `opl_silence` keys off all nine channels and clears the rhythm register `0xBD`. `snd_init` enables waveform select (register 1 to `0x20`).

Channel use, fixed by the converters (`m4/sound/fizz-dos/music.ts`, `sfx.ts`):

| Channels | Use |
|---|---|
| 0–4 | music melody |
| 5 | sound effects (`EFFECT_CHANNEL`); the driver writes `0xB5 = 0` (key off) before starting an effect |
| 6–8 | rhythm-mode drums (bass drum, snare and hi-hat, tom and cymbal), switched on by the song through `0xBD` |

The music stream loops; an effect stream plays once. Music is chosen by `choose_music` in `main.c` from the simulation's mode, level and boss health through `snd_music_for` (title, map, the level's song, or the boss song while a boss has health); effects come from the simulation's caption events through `snd_caption` (in `shell.rs`) and from menu actions through `snd_fx`. Requests are single volatile words that the 560 Hz handler takes up, so the main program never writes to the chip while the handler might.

**PC speaker.** `speaker_set` puts PIT channel 2 in square-wave mode (`outportb(0x43, 0xB6)`), loads the divisor low byte then high byte through port `0x42`, and sets bits 0 and 1 of port `0x61` (gate and speaker enable); `speaker_off` clears those two bits. Only effects play on the speaker; there is no speaker music.

**Choices.** `SND_*` in `m4/c/sound.h`:

| Choice | Music | Effects | Without an OPL2 |
|---|---|---|---|
| Auto (0) | OPL2 | OPL2 | speaker effects |
| AdLib (1) | OPL2 | OPL2 | off |
| Mixed (4) | OPL2 | speaker | speaker effects |
| Speaker (2) | none | speaker | speaker effects (no OPL2 detection) |
| Off (3) | none | none | — |

`snd_init` returns the mode actually in use. The Options screen can toggle music and effects and switch the effects between OPL2 and speaker (`snd_set_fx_device`, which falls back to the speaker when there is no OPL2). The Sound Blaster probe is shown on the system check only; there is no digitised sound.

## Files

All files are in the current directory, through `m4/c/save.c` (`fopen`, `fread`, `fwrite`). A slot number picks the name:

| Slot | File | Size | Written |
|---|---|---|---|
| −1 | `FIZZ.CFG` | 12 bytes | after each change on the Options screen and by `FIZZ savecfg` |
| 0 | `AUTO.SAV` | 48 bytes | on every visit to the map (`autosave` in `shell.rs`), as the web build does |
| 1–4 | `SLOT1.SAV` … `SLOT4.SAV` | 48 bytes | from the Save list |
| 5 | `QUICK.SAV` | 48 bytes | F5 in a level, on the map or in the pause menu; F9 loads it (`quick_keys` in `shell.rs`) |

Names are 8.3, upper case. `save_stamp` is `time(NULL)`; dates in the load list are shown as UTC (`save::date`), since a DOS clock has no time zone.

**Save (`m2/dos-rt/src/save.rs`).** Little endian; a file that is not exactly 48 bytes with the magic and version is treated as empty.

| Offset | Field |
|---|---|
| 0 | `FZSV` |
| 4 | version (1); 5–7 zero |
| 8 | `u32` saved at (seconds since 1970) |
| 12 | lives (`i32`) |
| 16 | score (`i32`) |
| 20 | next extra life (`i32`) |
| 24 | Fizz (ammunition, `i32`) |
| 28 | levels cleared mask (bit 15 is the secret level) |
| 32 | has a map position |
| 36, 40 | map x, map y (`f32` bits) |
| 44 | seconds played |

Loading starts a new game and sets these through the simulation's `state_set`; the load list shows levels cleared (bits 0–14), score, lives and date.

**Options (`FIZZ.CFG`, `settings_load` and `settings_save` in `main.c`).**

| Byte | Field |
|---|---|
| 0–2 | `FZC` |
| 3 | version (1) |
| 4 | music on |
| 5 | effects on |
| 6 | effects device (1 OPL2, 2 speaker) |
| 7 | frame rate (0 auto, 1 60 Hz, 2 30 Hz) |
| 8 | detail (0 auto, 1 high, 2 low) |
| 9 | night |
| 10 | sound choice: written as 0 (auto) and not read, as the start-up screen no longer offers one |
| 11 | spare |

Out-of-range values fall back to auto. Command-line arguments override the file for that run only.

## The C/Rust boundary

All calls are plain `extern "C"` with 32-bit integers, `f32` and raw pointers; there are no callbacks and no shared structures beyond byte buffers. Rust's symbols get DJGPP's leading underscore from the target (see [RUST_ON_DOS.md](RUST_ON_DOS.md)).

**Exported by Rust** (`m2/dos-rt/src/lib.rs`, `#[no_mangle]`):

| Function | Purpose |
|---|---|
| `fizz_m0(ticks, use53)` | the determinism run: 16 levels, a state hash every 60 ticks (used by `m2/c/main.c` for `m2/build.sh`) |
| `fizz_start(level, use53)` | sets the x87 to 53 bits, starts a new game at the title (255), map (254) or a level, fills the sim's sprite size table |
| `fizz_scripted(tick)` | input bits of the scripted walk, for repeatable runs |
| `fizz_tick(held)` | one 60 Hz tick through the shell with the given input bits |
| `fizz_goto(level)` | jumps to a level (wraps round the table) |
| `fizz_level()` | the current level |
| `fizz_draw(frame, alpha)` | build and paint in one call (`FIZZ.EXE` calls the two halves instead) |
| `fizz_build(alpha)` | the sim writes its interpolated draw list; returns the instance count |
| `fizz_paint(frame, count)` | draws the scene and the shell into the 64,000-byte frame; returns the clear colour |
| `fizz_night(on)` | night mode (darker ambient light) |
| `fizz_detail(low)` | low detail (no backdrop layers) |
| `fizz_boss_hp()` | the boss's remaining health, for the music |
| `fizz_sound_data()` | pointer to `sound.bin` in the EXE |
| `fizz_sound_len()` | its length |
| `fizz_mode()` | the sim's mode: 0 none, 1 attract (title), 2 map, 3 level |
| `fizz_goto_map()` | back to the overworld |
| `fizz_press(mode)` | a check: steer the scripted walk to a map marker and press, or not |
| `fizz_preview(frame, values, cpu)` | draws the system-check screen from the probe values |
| `fizz_preview_ready(values)` | whether the machine has what the game needs: a VGA, an x87 and enough memory |
| `fizz_preview_text(values, cpu, out, cap)` | writes the system-check screen as text with colour codes, for a machine without a VGA |
| `fizz_quit()` | whether the player chose Quit from a menu |
| `fizz_show(n)` | a check: opens a menu or screen directly (`FIZZ.EXE`'s `pause`, `options`, `ending`, `cineN`, `titles` and the like; 200 is the opening titles from their first tick) |

**Called by Rust, provided by C:**

| Function | File | Called from | Purpose |
|---|---|---|---|
| `malloc`, `free`, `realloc` | DJGPP libc | `lib.rs` | the global allocator |
| `printf`, `fflush` | DJGPP libc | `lib.rs` | panic message and the determinism run's output |
| `snd_caption(id)` | `sound.c` | `shell.rs` | plays the effect for a sim caption |
| `snd_fx(effect)` | `sound.c` | `shell.rs` | plays one effect by number |
| `snd_set_music(on)`, `snd_music_enabled()` | `sound.c` | `shell.rs` | music toggle |
| `snd_set_fx(on)`, `snd_fx_enabled()` | `sound.c` | `shell.rs` | effects toggle |
| `snd_have_opl()` | `sound.c` | `shell.rs` | whether an OPL2 was found (offers the device choice) |
| `snd_current_song()`, `snd_music_clock()` | `sound.c` | `shell.rs` | the song playing, and its clock in 560 Hz ticks since the stream began (through its loops; -1 when nothing plays on the OPL2 or a change of song is waiting for the handler): the opening titles follow it and the idle wait on the title menu counts by it |
| `snd_fx_device()`, `snd_set_fx_device(dev)` | `sound.c` | `shell.rs` | effects on OPL2 (1) or speaker (2) |
| `rate_mode()`, `rate_set(mode)`, `rate_active()` | `main.c` | `shell.rs` | frame-rate setting and the rate in use |
| `detail_mode()`, `detail_set(mode)`, `detail_active()` | `main.c` | `shell.rs` | detail setting and whether low detail is in use |
| `settings_save(night)` | `main.c` | `shell.rs` | writes `FIZZ.CFG` (night lives in the game, so Rust passes it) |
| `save_write(slot, data, len)` | `save.c` | `save.rs` | writes a save or the options |
| `save_read(slot, data, len)` | `save.c` | `save.rs` | reads one; returns the bytes read |
| `save_stamp()` | `save.c` | `save.rs` | seconds since 1970 |

The C side also calls `snd_music_for`, `snd_music`, `snd_init`, `snd_shutdown`, `snd_ticks` and `snd_mode` itself; `m5/bench` stubs every C function above so the shell runs natively.

## The browser build

`tools/make-site.sh` builds `FIZZ.EXE`, then packs a js-dos 8 bundle, `site/game.jsdos`: a zip holding `FIZZ.EXE`, `CWSDPMI.EXE` and its documentation, `TONETEST.EXE` (a speaker and OPL test tone built from `tools/dos/tonetest.c`), `.jsdos/jsdos.json` and `.jsdos/dosbox.conf` copied from `site/dosbox.web.conf`. Two more bundles, `game-default.jsdos` and `game-compat.jsdos`, differ only in `oplemu`; `site/player.js` loads one of them with `?opl=default` or `?opl=compat`. js-dos itself is vendored under `site/vendor/js-dos/` by `tools/vendor-jsdos.sh`.

The bundles are not committed. The live page serves a release's browser bundle: `.github/workflows/pages.yml` downloads it from the release, `tools/stage-site.sh` makes it `site/game.jsdos`, derives the two comparison bundles by changing the `oplemu` line in its `dosbox.conf` and puts the tag in the footer, and the workflow deploys `site/`. The release workflow calls it after it publishes, so a release updates the page; a push to `main` that changes `site/` publishes it again with the newest release's game. A change to the game therefore reaches the page with the next release, not with its merge.

`site/dosbox.web.conf` differs from the desktop `tools/dosbox.conf` in that it has sound on (`sb2` at `0x220`, IRQ 7, DMA 1, `oplmode = opl2`, `oplemu = fast`), a 44,100 Hz mixer and an autoexec that mounts the bundle and runs `fizz.exe`. Its key line is `prebuffer = 0`: js-dos 8's mixer pushes audio to the page in 512-sample blocks and speeds up by three times the prebuffer to fill its buffer, so at 40 ms the stream carried about 12% more samples than its rate, more than the page's time-stretcher (5% at most) can absorb, and samples were dropped continuously (the PC speaker's 440 Hz test tone measured 393 Hz). At 0 the stream runs at its real rate. [PERFORMANCE.md](PERFORMANCE.md#in-the-browser) has the measurements.

## Not verified

Nothing here has run on real hardware: the OPL2 write delays, the retrace wait, the copy to an ISA VGA card, the keyboard on a real controller and CWSDPMI paging have only run in DOSBox Staging and js-dos. Audio cannot be checked headless; see [STATUS.md](STATUS.md).
