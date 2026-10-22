# Changelog

What changed in each release of Lieutenant Fizz for DOS. A release is a tag `vX.Y.Z`; `tools/make-release.sh` builds its files and `.github/workflows/release.yml` publishes them. The simulation is the main game's and does not change between releases.

## Unreleased

### Added

- Opening titles: Liminal HQ presents, the *Lieutenant Fizz* wordmark and a chase, with a new score. They play when the game starts, any key skips them, and they come back for two loops when the title menu has sat untouched for two loops of its song.
- `fx`, a small module of screen effects (fades, a flash, easings and tweens in fixed point, a cue timeline) that the titles use and the shell's fades now run on.
- A music clock in the sound driver, an any-key input bit, and `FIZZ.EXE soundtestN` to play any extra song headless.

## 0.1.0 — 2026-10-21

The first release. Episode 1 plays from the title through the overworld and every level, with boss dialogue, saves and options. Still missing: the opening cinematic, the ending and credits, and the controls screen. Nothing has run on real hardware yet; see `docs/STATUS.md`.

### Added

- The game as a 32-bit DOS program: the main game's simulation (checked against the web build's 800 state hashes), a C layer for VGA mode 13h, the keyboard and AdLib or PC-speaker sound, and a Rust runtime for the drawing, menus, saves and HUD.
- A start-up system check, title, pause menu, options, load and save lists, and result cards.
- A browser page on GitHub Pages that runs the game through js-dos, with sound, fullscreen and Restart buttons.
- Download saves and Upload saves buttons under the emulator, so a player can keep their saves across a Restart.
- A sound test tone, `TONETEST`, for checking AdLib and the PC speaker.
- Release files: a 1.44 MB floppy image, the disk's contents as a zip, a DOSBox demo zip and the browser bundle.
- Link preview tags and a share image for the page.
- Guides: Rust on DOS, the DOS architecture with an animated diagram, and a performance write-up.
- Measuring tools in `m5/bench`: instruction counts, frame checksums, callgrind profiling and an estimate of 486 cycles.

### Changed

- The renderer is faster: baked tiles, sprite placement in integers, lighting in fixed point, and a draw list that is kept while the scene is still. Instructions a frame fell by 23% to 59% in the benchmark scenes; the pictures are the same, apart from lighting that can differ by one level on at most 0.13% of pixels.
- The browser page's audio runs at its real rate (the mixer's prebuffer is 0) and uses the `fast` OPL emulator; the lighter `compat` one played the music wrongly.
- The page's workflows use action versions that run on Node 24.

### Fixed

- Quitting says goodbye instead of printing run statistics.
