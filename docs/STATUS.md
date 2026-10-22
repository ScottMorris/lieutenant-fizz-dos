# Status: Episode 1 for DOS

Where the DOS port stands against the main game (`liminal-hq/lieutenant-fizz`, Episode 1: The Cocoa Caper). The port runs the game's own simulation, so the rules, levels, enemies and bosses are the main game's; what differs is everything around the simulation: the screens, the drawing, the sound and the input.

## What plays

- **The game:** the title with its attract loop, the overworld, every level in the simulation (0 to 15), the boss fights with their dialogue, lives, score, Fizz ammo and extra lives.
- **The shell:** a start-up system check (video, input, sound and maths hardware, and a speed test of the real renderer), title, pause, options, load and save lists, a boxed HUD, map prompts, floating caption words, toasts and result cards, a controls screen (from the title and the pause menu) and a key hint under each menu, and Ben on the title wordmark (he bobs, hops up and pogoes across it, turns to the menu and waves on a pick), with fades between screens.
- **Saves and options:** progress is written to `AUTO.SAV` on every map visit and to `SLOT1.SAV` to `SLOT4.SAV` from the pause menu. F5 and F9 quick-save and quick-load in `QUICK.SAV`, a slot of its own (the main game writes the newest manual slot instead). Saves are progress-only, which is also the main game's behaviour. The options are written to `FIZZ.CFG`, a separate file, and read at the next start.
- **Sound:** AdLib (OPL2) music and effects, PC-speaker effects, or the music on AdLib with the effects on the speaker. The music and effects are converted from the main game's Undertone data.
- **Speed options:** frame rate (auto, 60 Hz, 30 Hz) and detail (auto, high, low) in Options, with command-line overrides.
- **Releases:** each tagged release carries a 1.44 MB floppy image (label `FIZZ EP1`, serial number `FE01-0010` for 0.1.0: the game, then the version), the same files as a zip, a DOSBox demo zip, the browser bundle and `SHA256SUMS`. The image's file system is checked and the game runs from it in DOSBox; no image has been booted or written to a real floppy.

## Not ported yet

- The eight-scene opening cinematic, drawn in TypeScript in the main game and not part of the simulation.
- The ending: the closing panels, the credits roll and the stinger. The port shows result cards but has no ending sequence.
- Play samples and replays.

## Different on purpose

- **Lighting is brightness only.** The palette holds 16 EGA colours at 14 brightness levels, so a coloured light dims or brightens a pixel but cannot tint it. Sprite tints snap to the nearest EGA colour.
- **The hints are the menu hint lines and the controls screen.** The main game has no separate in-level tutorial, so there is nothing more to port.
- **Ben on the title does not lean**, as he does in the web build: there is no rotated blit for the interface.
- **No alpha blending.** A sprite is drawn or it is skipped.
- **The view is 20 × 12.5 tiles** on a 320 × 200 screen; the web build shows about 13 tiles of height.
- **Sound is converted, not re-synthesised.** The OPL2 and PC-speaker versions are approximations of the Undertone music and effects.
- **No touch, fullscreen or launcher.** Those belong to the web and app builds.
- **Detail low** leaves out the backdrop layers (the sprites named `...Back`): the red curtain in the Sugar Glass Gallery, and a dim wall in the caves.

## Verified

- **Determinism.** `m2/build.sh` runs 16 levels for 3000 ticks each on one scripted input stream, hashes the simulation's 26 state slots every 60 ticks and compares the 800 hashes with the web build's wasm. They are identical, with the x87 precision control set to 53 bits. At the default 64-bit precision they diverge at level 0, tick 120.
- **Frames.** `m5/bench` prints a checksum of the last frame of five scenes (the map, the day level, the cave, the cave at night and level 9). Speed changes keep them identical.
- **Menus.** A pause-menu Resume with Enter held does not enter the level on the map; the start-up screen's Enter does not reach the title.
- **Options.** A `FIZZ.CFG` is read and shown on the Options screen, and options saved from the command line load again.

## Not verified

- **A full playthrough.** The parity run walks each level on a script; it does not clear them. No one has played Episode 1 from the title to the credits on the DOS build.
- **Real hardware.** Everything has run in DOSBox. The 486 figures below are DOSBox at a fixed cycle count, which counts instructions evenly; a real 486 is likely slower.
- **Audio.** The sound has been listened to only by the owner on a laptop, in DOSBox. Nothing here can check it headless.
- **Keyboard feel in other emulators and DOS versions.** The keyboard is read through an IRQ 1 handler that has been tried in DOSBox Staging only.

## Speed

Instructions per frame for a sim step, the build and the drawing, counted natively on a 32-bit x87 build (`m5/bench`), and milliseconds per frame in DOSBox at `core = normal`, `cycles = fixed 26800` (the usual stand-in for a 486DX2-66). The rate counts the build, the paint, the copy to the screen and one sim step. `docs/PERFORMANCE.md` has the measurements behind these, the figures before the speed work, and what is left to do.

| Scene | Instructions a frame | Build | Paint | Approximate rate |
|---|---|---|---|---|
| Day level | 0.72 M | 4.4 ms | 23 ms | 35 fps |
| Overworld map | 1.16 M | 12 ms | 31 ms | 23 fps |
| Crystal Caves, detail high | 1.15 M | 7.0 ms | 36 ms | 23 fps |
| Crystal Caves, detail low | — | 7.0 ms | 26 ms | 29 fps |
| Crystal Caves at night | 1.24 M | 7.0 ms | 39 ms | 21 fps |
| Level 9, detail high | — | 7.8 ms | 39 ms | 21 fps |
| Level 9, detail low | — | 7.8 ms | 31 ms | 25 fps |

A 486DX2-66 has about 1.1 M cycles a frame at 60 fps and 1.9 M at 35 fps. DOSBox counts about one cycle an instruction, which flatters x87 work: weighted by the 486's own clock counts, a cave frame is about 2.4 M cycles, so a real DX2-66 is likely slower than the table. The sim's step is about 5% of such a machine at 60 ticks a second. A 30 Hz cap gives a steady pace on machines that can do about 30 to 55 fps; it does not speed up slower ones.

## Ideas not built

- A bootable FreeDOS image for real hardware and 86Box.
- Replaying the main game's playtest bot solutions through the DOS build and comparing hashes, to show every level can be cleared.
- PC-speaker music.
- The speed work listed under "Next opportunities" in `docs/PERFORMANCE.md`: an integer draw list from the sim, shading while copying to the screen, skipping the backdrop under opaque tiles, fades through the palette.
