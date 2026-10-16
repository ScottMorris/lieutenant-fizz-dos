# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

Lieutenant Fizz for DOS (`ScottMorris/lieutenant-fizz-dos`) ports *Lieutenant Fizz — Episode 1: The Cocoa Caper* to MS-DOS: the game's own Rust simulation, built for a 32-bit x87 DOS target and linked with a small C layer (DJGPP) that draws in VGA mode 13h, reads the keyboard and drives AdLib and the PC speaker. It fits a 1.44 MB floppy, runs in DOSBox and runs in a browser on GitHub Pages through js-dos. See `AGENTS.md` for the authoritative contributor conventions — most importantly: **Canadian English** spelling everywhere; **Conventional Commits** for commit messages but **never in PR titles**; **merge commits only** when merging PRs; the licence/copyright header on new source files; the **authoring voice** (docs state facts for a later reader, never how the conversation went); and **no pushes unless explicitly asked**.

`docs/STATUS.md` (what is and is not ported) and `docs/ARCHITECTURE.md` (layers, build, determinism) are the reference. The main game, `liminal-hq/lieutenant-fizz`, owns the design and the simulation; this repository carries a patched `no_std` copy of the simulation and must not change its rules.

## Layout

- `m0` native determinism harness; `m1` first Rust-in-DOS proof; `m2` the simulation as a DOS program (`sim`, `game`, `dos-rt`, `dos.json`); `m3` sprite export; `m4` the playable build (C layer in `m4/c`); `m5/bench` instruction-count bench
- `tools/` fetch, run, pack and convert scripts, `dosbox.conf`; `dist/` the demo folder; `site/` the Pages page and js-dos bundle; `docs/`; `assets/`
- `vendor/` (DJGPP, DOSBox Staging, CWSDPMI), `m2/target` and `out/` are local only and ignored

## Commands

```bash
tools/fetch-tools.sh                 # toolchain into vendor/
m2/build.sh                          # build the simulation for DOS and compare with the web build (PASS: 800 checkpoints)
m4/build.sh --no-run                 # build the playable m4/c/fizz.exe
tools/dosrun.sh <dir> "<command>"    # run a DOS program headless in DOSBox
tools/make-dist.sh                   # pack dist/fizz-dos-demo.zip
tools/make-site.sh                   # build the GitHub Pages bundle
```

The scripts are the source of truth. Run `m2/build.sh` after any change to the simulation, the drawing code or `m2/dos-rt`; it must print `PASS`.

## Architecture — the key things to understand

**The simulation is the main game's, and it must stay identical.** `m2/sim` and `m2/game` are patched copies of `crates/sim` and `episodes/episode-1/game` (`no_std`, `fmath` traits over `libm`). The x87 precision control is set to 53 bits before the first tick, or the results drift from the web build within two seconds. Never reorder floating-point operations in the simulation for speed.

**The frame is a palette trick.** A 320×200 byte buffer; `byte = level * 16 + colour` (16 EGA colours at 14 brightness levels), slot 255 is the level's clear colour. Sprites are blitted with fixed point (no per-pixel floats); the light for each 8×8 cell comes from the sim's lights and a shade pass turns markers into levels. Speed is measured with `m5/bench` (instructions per frame) and then DOSBox at `cycles=fixed 26800`.

**C owns the machine, Rust owns the game.** `m4/c` hooks IRQ 1 (keyboard) and IRQ 0 (560 Hz timer for OPL2 and PC-speaker sound), sets mode 13h and copies the frame to the screen; `m2/dos-rt` does the drawing, the screens, menus, saves and the HUD, and exports `fizz_*` functions over a raw C ABI. Options live in `FIZZ.CFG`, apart from the `*.SAV` game saves.

## Conventions (from AGENTS.md)

- **PR titles**: human-readable, imperative, sentence case, ~70 chars, no Conventional Commit prefix. Descriptions use `## Summary` + `## Test plan`. Every PR gets a category label plus scope labels.
- **Commits**: Conventional Commits with markdown bodies (what/why), hard-wrapped; write bodies to a file and use `git commit -F`. Stage with `git add <specific paths>`, never `-A` or `-a`. End with the `Co-Authored-By` trailer.
- **Merging**: merge commits only, and only after the user's explicit go-ahead.
- **Licence headers** on new or substantially rewritten `.rs`/`.c`/`.h`/`.ts`/`.js`/`.py` files (one-line summary + `(c) Copyright 2026 Liminal HQ, Scott Morris` + `SPDX-License-Identifier: Apache-2.0 OR MIT`).
- **No hard wrapping** in markdown prose; use a real em dash (`—`), never `--`.
- **Authoring voice**: comments and docs state facts directly; no "this PR", "we tried" or reviewer references.
- **Git**: never push (especially force-push) unless explicitly asked; prefer the `gh` CLI for GitHub work.
- **Verification**: say what was and was not run. Nothing here has run on real hardware, and audio cannot be checked headless.

Keep this file and `AGENTS.md` in sync: when a convention changes there, update the summary here in the same PR.
