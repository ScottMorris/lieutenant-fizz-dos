# Changelog

What changed in each release of Lieutenant Fizz for DOS. A release is a tag `vX.Y.Z`; `tools/make-release.sh` builds its files and `.github/workflows/release.yml` publishes them. The simulation is the main game's and does not change between releases.

## Unreleased

### Changed

- The start-up system check is read-only and shows what the game needs. Video lists the VGA adapter, the 320 × 200 × 256 mode (or, without a VGA, the best mode the machine has: EGA, CGA or monochrome text) and the 14 lighting shades; Input the keyboard, mouse and joystick; Sound the PC speaker, AdLib and Sound Blaster; Machine the processor, the x87 and free memory. Without a VGA, an x87 or 10 MB of free memory the speed test is not run, the footer names what is missing and only Esc leaves. The A, M, P and O sound keys are gone; the sound is set on the Options screen, and the sound choice kept in `FIZZ.CFG` is no longer read.
- The game no longer sets VGA mode 13h on a machine without a VGA: the system check is written on the text screen instead.
- The title plate no longer carries the "work in progress" legend.

## 0.2.1 — 2026-10-22

The game is the same as 0.2.0, 24% smaller. Nothing has run on real hardware yet; see `docs/STATUS.md`.

### Added

- A debug zip with each release (`-debug.zip`), holding the unstripped `FIZZ.EXE` for reading a crash's traceback.

### Changed

- The game in a release and in the browser bundle is stripped of its symbols and debugging sections: `FIZZ.EXE` is 853,504 bytes, 24% smaller than 1,123,051, and the floppy has 396,288 bytes free instead of 126,464. The program is the same: its code and data bytes are unchanged, and in DOSBox it draws the same frames and drives the sound the same way. The checks and `tools/dosrun.sh` still run the unstripped build.
- A release updates the page: it serves the release's browser bundle and shows the release's version in its footer, and the game bundles are no longer committed.
- The floppy image's creation time is the time its files carry. It was fixed at a constant in 2015 by `mkfs.fat --invariant`, while the files were stamped with the build's time. The files, the zips, the image and the browser bundle now all carry the one time (the last commit's, or `SOURCE_DATE_EPOCH`, in UTC), so a build gives the same files in any time zone.

## 0.2.0 — 2026-10-22

Everything the main game has in Episode 1 is in: the opening titles, the opening story, the ending and credits, the controls screen, quick-save and Ben on the title. Nothing has run on real hardware yet, and the sound has been listened to only in DOSBox and the browser; see `docs/STATUS.md`.

### Added

- Opening titles: Liminal HQ presents, the *Lieutenant Fizz* wordmark and a chase with Ben stunning the Zargs, over a score of its own, 12.8 s in mode 13h. They play when the game starts and any key skips them. When the title menu has sat untouched for two loops of its song (about two minutes) they play again for two loops, then the menu returns. Every frame is checked byte for byte against a port of the design's drawing routine.
- The opening story from New Game: eight letterboxed scenes (the backyard, the lab, the liftoff, the flight and the landing) with the typed text and each scene's music, then the map with a word about where to start. Enter moves on and Esc skips it.
- The ending: four letterboxed panels after the last boss, the credits roll (Enter speeds it up, Esc skips) with the ending's music, and an end card that returns to the title.
- Ben on the title wordmark: he bobs, leans, hops up and pogoes across it, turns to the menu when the cursor moves and waves on a pick.
- A Controls screen on the title and the pause menu, and a key hint under each menu.
- Quick-save and quick-load on F5 and F9, in a slot of their own (`QUICK.SAV`) so the four slots picked from the pause menu are never overwritten. The main game quick-saves into the newest manual slot; the proposal to change that is liminal-hq/lieutenant-fizz#220.
- `fx`, a small module of screen effects (fades, a flash, easings and tweens in fixed point, a cue timeline) that the titles use and the shell's fades now run on.
- A music clock in the sound driver, an any-key input bit, and `FIZZ.EXE soundtestN` to play any extra song headless.
- A release's notes now list the pull requests merged since the last release, grouped by label.

### Changed

- A save is written to a temporary file and then renamed, so a failed write keeps the old save.

### Fixed

- The disk's `README.TXT` and `README.RTF` no longer list the opening, the ending, the credits and the controls screen as missing.

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
