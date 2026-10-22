# Status: Episode 1 for DOS

Where the DOS port stands against the main game (`liminal-hq/lieutenant-fizz`, Episode 1: The Cocoa Caper). The port runs the game's own simulation, so the rules, levels, enemies and bosses are the main game's; what differs is everything around the simulation: the screens, the drawing, the sound and the input.

## What plays

- **The game:** the title with its attract loop, the overworld, every level in the simulation (0 to 15), the boss fights with their dialogue, lives, score, Fizz ammo and extra lives.
- **The shell:** a start-up system check, title, pause, options, load and save lists, a boxed HUD, map prompts, floating caption words, toasts and result cards, a controls screen (from the title and the pause menu) and a key hint under each menu, and Ben on the title wordmark (he bobs, leans, hops up and pogoes across it, turns to the menu and waves on a pick), with fades between screens.
- **The start-up check:** a read-only screen of what the machine has for the game, in four panels: Video (a VGA adapter, the 320 × 200 × 256 mode — on a machine without a VGA, the best mode it has, from EGA 640 × 350 × 16 down to 80 × 25 mono text — and the 14 lighting shades that come with the VGA), Input (keyboard, mouse, joystick; the game uses neither of the last two yet), Sound (PC speaker, AdLib, Sound Blaster) and Machine (the processor's family code, the x87 and free memory). A speed test of the real renderer follows. Free memory of 10 MB or more is ticked; from 1 MB to just under 10 MB the row is a yellow warning (the game starts, but CWSDPMI pages to its swap file, so it is slow). Without a VGA, an x87 or 1 MB of free memory the game does not start: the speed test is not run and the footer names what is missing, and only Esc leaves. Mode 13h is only set on a VGA; without one the check is written on the text screen. The sound is chosen on the Options screen.
- **The opening titles:** the game opens with Liminal HQ presents, the *Lieutenant Fizz* wordmark and a chase (Ben stunning the Zargs with fizz bubbles) over a score of its own, 12.8 s in mode 13h, then the title menu. Any key skips them. They play again, for two loops, when the title menu has sat untouched for two loops of its song (about two minutes); any key, or leaving the menu and coming back, starts that wait again. The picture is checked against the design's own drawing routine, ported as an oracle, frame for frame.
- **The opening:** New Game from the title plays the eight-scene cinematic (the backyard, the lab, the liftoff, the flight and the landing) as letterboxed pictures with the typed text and the scene's music, then goes to the map with a word about where to start. Esc skips it. Out of lives still starts a new game without it.
- **The ending:** after the last boss, four letterboxed panels typed out at the main game's speed, the credits roll (Enter speeds it up, Esc skips) with the ending's music, and an end card that returns to the title.
- **Saves and options:** progress is written to `AUTO.SAV` on every map visit and to `SLOT1.SAV` to `SLOT4.SAV` from the pause menu. F5 and F9 quick-save and quick-load in `QUICK.SAV`, a slot of its own (the main game writes the newest manual slot instead). Saves are progress-only, which is also the main game's behaviour. The options are written to `FIZZ.CFG`, a separate file, and read at the next start.
- **Sound:** AdLib (OPL2) music and effects, PC-speaker effects, or the music on AdLib with the effects on the speaker. Auto, the default, takes the AdLib for both when it is there and the speaker for the effects otherwise. Options offers only what applies: Music and "Effects play on" with an AdLib, Effects whenever sound runs; the choices, and the M and E keys', are kept in `FIZZ.CFG` and read at the next start, and a run without the hardware keeps the choice made on one with it. The Sound Blaster is detected but not used (there is no digitised sound), so it is not a setting. The music and effects are converted from the main game's Undertone data.
- **Speed options:** frame rate (auto, 60 Hz, 30 Hz) and detail (auto, high, low) in Options, with command-line overrides.
- **Releases:** each tagged release carries a 1.44 MB floppy image (label `FIZZ EP1`, serial number `FE01-0010` for 0.1.0: the game, then the version), the same files as a zip, a DOSBox demo zip, the browser bundle, a debug zip with the unstripped game and `SHA256SUMS`; the game in all but the debug zip is stripped of its symbols and debugging sections (853,504 bytes, from 1,123,051), which leaves the floppy 396,288 bytes free; the release then updates the live page with that bundle and its version. One time stamps the files in the image, the zips and the browser bundle and the image's own creation time (the last commit's time, or `SOURCE_DATE_EPOCH`, read as UTC), so the same tree gives the same files wherever it is built; `tools/make-release.sh` checks it. The image's file system is checked and the game runs from it in DOSBox; no image has been booted or written to a real floppy.

## Not ported yet

Nothing of Episode 1's play or presentation is left. The web and app builds' play samples, replays, touch controls and fullscreen are listed under "Different on purpose".

## Different on purpose

- **No stinger.** Episode 1 does not ship the main game's post-credits stinger (it belongs after Episode 3), so the port has none.
- **No play samples or replays.** Those are the web and app builds' telemetry and viewer, built on browser storage.
- **The opening's pictures are the main game's sprites at half the tile size**, so the 320 × 200 screen shows 40 × 25 tiles. A tint picks the nearest EGA colour and its brightness a palette level (the night's dark blues are dim blues); alpha is a threshold, so a faint glow or a fade is either drawn or left out.
- **The titles' skip cuts the music at the black frame** rather than fading its volume: the notes key off when the menu comes up and the title song begins. A volume fade would need the sound handler to scale the chip's operator levels.
- **The titles' picture is not scanlined.** The design's preview has an optional CRT overlay; the game draws plain pixels.
- **The ending's panels sit over the dimmed last frame of the game**, and the credits roll is stacked (a role above its name) so the long roles fit 320 pixels.
- **Lighting is brightness only.** The palette holds 16 EGA colours at 14 brightness levels, so a coloured light dims or brightens a pixel but cannot tint it. Sprite tints snap to the nearest EGA colour.
- **The hints are the menu hint lines and the controls screen.** The main game has no separate in-level tutorial, so there is nothing more to port.
- **No alpha blending.** A sprite is drawn or it is skipped.
- **The view is 20 × 12.5 tiles** on a 320 × 200 screen; the web build shows about 13 tiles of height.
- **Sound is converted, not re-synthesised.** The OPL2 and PC-speaker versions are approximations of the Undertone music and effects.
- **No touch, fullscreen or launcher.** Those belong to the web and app builds.
- **Detail low** leaves out the backdrop layers (the sprites named `...Back`): the red curtain in the Sugar Glass Gallery, and a dim wall in the caves.

## Verified

- **Determinism.** `m2/build.sh` runs 16 levels for 3000 ticks each on one scripted input stream, hashes the simulation's 26 state slots every 60 ticks and compares the 800 hashes with the web build's wasm. They are identical, with the x87 precision control set to 53 bits. At the default 64-bit precision they diverge at level 0, tick 120.
- **Frames.** `m5/bench` prints a checksum of the last frame of five scenes (the map, the day level, the cave, the cave at night and level 9). Speed changes keep them identical.
- **The titles.** All 768 frames of one loop from the native bench, and 17 frames of the DOS build in DOSBox, are byte-identical to the oracle's. The flow (a loop and then the menu, a key's fade, the idle wait ending, two loops and then the menu, a key moving the wait back) is run by the bench's `FLOWTEST` with the music clock unavailable, so by game ticks.
- **Menus.** A pause-menu Resume with Enter held does not enter the level on the map; the start-up screen's Enter does not reach the title. The bench's `FLOWTEST` turns night on with the Options row and then toggles it the way the N key does: off, then on again.
- **Options.** A `FIZZ.CFG` is read and shown on the Options screen, and options saved from the command line load again.
- **The start-up check's cases.** The drawn screen was captured headless in DOSBox Staging on its default VGA machine, with `machine = ega`, `cga` and `hercules` (each read as its adapter, the game not started) and with `memsize = 16`, `8`, `4`, `2` and `1` (13 MB ticked; 5 MB and 1 MB a warning, the speed test run and Enter offered; under 1 MB with `memsize = 2` and `1`, refused). The scripted game, with sound, ran 300 ticks with `memsize = 4`; with `memsize = 2` DOSBox stopped when the sound handler met a paged-out page, and with `memsize = 1` the program ended without its report, so the refused cases are the ones that fail. The processor row read `i386`, `i486` and `i586` with `cputype = 386`, `486` and `pentium`. A monochrome VGA and a missing x87 cannot be set up in DOSBox Staging; those rows were drawn from forced probe values. The text form of the check on a machine without a VGA was read from standard output; its colours on a real text screen were not looked at.

## Not verified

- **A full playthrough.** The parity run walks each level on a script; it does not clear them. No one has played Episode 1 from the title to the credits on the DOS build.
- **Real hardware.** Everything has run in DOSBox. The 486 figures below are DOSBox at a fixed cycle count, which counts instructions evenly; a real 486 is likely slower.
- **The titles with sound and a keyboard.** The picture's lock to the music clock, the any-key skip and the idle wait counted by the song's clock were not run in DOSBox with keys, and the titles' score has not been listened to. Headless, only the stream's register writes and the clock reading (1679 after 1680 ticks) were checked.
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
