# AGENTS.md

## Table of Contents

- [Project Status](#project-status)
- [Source of Truth](#source-of-truth)
- [Localization and Spelling](#localization-and-spelling)
- [Markdown Formatting](#markdown-formatting)
- [Authoring Voice](#authoring-voice)
- [Commit Messages](#commit-messages)
- [Pull Request Titles](#pull-request-titles)
- [Pull Request Content](#pull-request-content)
- [Pull Request Labels](#pull-request-labels)
- [Git Workflow](#git-workflow)
- [Local Tooling](#local-tooling)
- [Code Conventions](#code-conventions)
- [Testing](#testing)
- [Documentation](#documentation)
- [Repository Layout](#repository-layout)
- [Licence and Copyright](#licence-and-copyright)
- [Agent Behaviour](#agent-behaviour)

## Project Status

Lieutenant Fizz for DOS (`ScottMorris/lieutenant-fizz-dos`) is a port of *Lieutenant Fizz — Episode 1: The Cocoa Caper* to MS-DOS. The game's own Rust simulation (from `liminal-hq/lieutenant-fizz`) is built for a 32-bit x87 DOS target and linked with a small C layer (DJGPP) that draws in VGA mode 13h, reads the keyboard and drives AdLib and the PC speaker. It fits a 1.44 MB floppy, runs in DOSBox, and runs in a browser through js-dos on GitHub Pages. The port is playable but not complete; `docs/STATUS.md` lists what is missing, what differs on purpose and what has not been verified.

## Source of Truth

- `docs/STATUS.md` is the port's status against the main game: not ported, different on purpose, not verified, ideas not built.
- `docs/ARCHITECTURE.md` describes the layers, the build pipeline and the rules that keep the simulation identical to the web build.
- `docs/DOS_ARCHITECTURE.md` is the machine side in detail: memory, start-up, the interrupt handlers, timing, VGA, sound, files and the C/Rust boundary; `docs/architecture.svg` is the animated diagram of the layers.
- `docs/RUST_ON_DOS.md` is the toolchain as a guide: the target spec, `-Zbuild-std`, `coff2go32.py`, linking with DJGPP, the runtime glue and the x87.
- `docs/PERFORMANCE.md` is where a frame's time goes on a 486, how it is measured (bench, callgrind, DOSBox, the 486 cycle estimate) and the speed work still open.
- The main repository, `liminal-hq/lieutenant-fizz`, holds the game's design (`docs/ENGINE_SPEC.md`, the episode's `GAME_DESIGN.md` and `STORY.md`). This repository changes the main game's rules only through the small patch that makes the simulation `no_std`; gameplay changes belong upstream.
- If code and docs disagree, resolve it deliberately: change the code to match, or update the doc in the same branch with the reasoning.

## Localization and Spelling

**REQUIREMENT:** All UI strings, code identifiers, comments, commit messages, pull request descriptions and documentation MUST use **Canadian English** spelling, unless exact external spelling is required by tooling, APIs, platform interfaces or published identifiers (for example the CSS `color` property or a DOS `COLOR` constant).

Examples:

- `colour` instead of `color`
- `centre` instead of `center`
- `neighbour` instead of `neighbor`
- `behaviour` instead of `behavior`
- `cancelled` instead of `canceled`
- `licence` (noun) vs `license` (verb)

## Markdown Formatting

**REQUIREMENT:** Do not hard-wrap markdown prose. Write each paragraph or bullet as a single unwrapped line in the source, no matter how long, and let the renderer reflow it. This applies to PR descriptions, docs under `docs/`, README files and any other markdown. Commit message bodies are the one exception: hard-wrap those (see [Commit Messages](#commit-messages)).

- This does not apply to genuinely separate list items, headings, or intentional line breaks.
- Code comments are not markdown-rendered and may wrap normally at a reasonable line length.

**Em dashes:** use a real em dash (`—`) in prose, never `--` as a substitute. This does not apply to a double-hyphen with another meaning (a CLI flag such as `--check`, a numeric range).

## Authoring Voice

**REQUIREMENT:** Ship the result, not how the conversation arrived at it. Write every outward-facing line (code comments, identifier names, docs, PR descriptions) as the author of the artefact, for the reader who will encounter it later.

- Don't reference "this PR", "the review", a reviewer's name, or a commit SHA inside code comments or PR prose. State the fact or reasoning directly.
- When a comment is edited more than once, rewrite it as one clean explanation rather than stacking fragments.
- Commit messages are the exception: they are the right place to record _why_ a change happened, including debugging context and measurements. The `## Test plan` section of a PR description is a similar exception.

## Commit Messages

**Format:** Use Conventional Commits (`feat:`, `fix:`, `docs:`, `test:`, `ci:`, `build:`, `chore:`, `refactor:`, `perf:`).

- Use `test:` for test-related changes, including fixes to tests themselves. Use `fix:` only when it fixes application code.
- Use `ci:` or `build:` when the primary change is workflow or build behaviour.
- Keep each commit focused on the specific change made in that commit; describe the now, not the whole project history.

**Body requirements:**

- Explain what and why (not how).
- Use markdown: **bold**, _italics_, `code`, flat bullet lists. No markdown headings; use **bold labels** if sections are needed.
- **Backtick every code-level reference**: function, type and variable names, file and directory paths, crate names, config keys and CLI flags (for example `m2/dos-rt/src/video.rs`, `FIZZ.CFG`, `--check`). Plain-English descriptions and in-game text use quotes, not backticks.
- Hard-wrap paragraphs at roughly 72-100 characters.
- End with the agent co-author trailer when an agent authored the commit.

**Shell interpolation safety:**

- Do not pass markdown-heavy bodies through `git commit -m "..."` when they contain backticks, `$()` or other shell-sensitive characters.
- Write the message to a file with a single-quoted heredoc and commit with `git commit -F <file>`.
- Verify with `git log -1 --pretty=fuller` and amend immediately if interpolation altered the text.

## Pull Request Titles

**REQUIREMENT:** PR titles are human-readable summaries of the change.

- Start with a capital letter, use the imperative mood, and keep to roughly 70 characters.
- Do not use Conventional Commit prefixes in PR titles.
- Describe the outcome or behaviour change, not internal process.
- Do not rename merged PRs unless explicitly requested.

## Pull Request Content

- Open pull requests ready for review by default. Use a draft only when asked or when there is a clearly communicated blocker.
- Default structure: `## Summary` (flat bullets with bold lead-ins) followed by `## Test plan` (checklist bullets with concrete commands, and an explicit statement of anything that could not be verified).
- Keep the summary focused on outcomes and behaviour changes, not commit chronology.

## Pull Request Labels

Add labels to every PR when it is created and keep them accurate as scope changes.

- **Primary category (at least one):** `enhancement`, `bug`, `documentation`, `testing`, `ci`, `build`, `chore`.
- **Scope:** `dos`, `graphics`, `sound`, `input`, `parity`, `rust`, `sim`, `rendering`, `audio`, `art`, `ui`, `performance`, `pages`, `deploy`, `site`, `design-docs`, `experimental`.
- Prefer GitHub category labels (`enhancement`, `bug`) over Conventional Commit terms (`feat`, `fix`).

## Git Workflow

**REQUIREMENT:** Do not push (especially force-push) unless explicitly requested by the user.

- **Merge commits only.** Pull requests are merged with a merge commit: never squash and never rebase-merge. Do not rewrite history that has been pushed. Never merge without the user's explicit go-ahead.
- **Branch naming:** `feat/<short-description>`, `docs/<short-description>`, `chore/<short-description>`; for fixes, `fix/issue-<number>-<short-description>`.
- **Focused commits:** when work splits naturally into implementation, validation and docs, use separate contextual commits.
- **Stage explicitly:** use `git add <specific paths>`, never `git add -A` or `git commit -a`. Retry on an `index.lock` collision rather than deleting the lock.
- Do not commit scratch files, build outputs (`*.exe`, `out/`, `target/`, `vendor/`) or large source archives.
- **Releases:** add the version's section to `CHANGELOG.md` in a PR and merge it; the tag `vX.Y.Z` is then pushed on the merge commit (only when the user asks), and `.github/workflows/release.yml` checks the tree, runs `tools/make-release.sh` and publishes the files with that section as the notes, followed by the pull requests since the last release grouped by their labels (`.github/release.yml`, the same file the other Liminal HQ products use; `skip-changelog` and `internal` leave a PR out), so every PR needs its category label. A 0.x release is a pre-release.
- **The build image:** the CI and the release run in `ghcr.io/liminal-hq/ci-dos`, the DOS tier of the shared images defined in `liminal-hq/.github` (`docker/ci/Dockerfile`), named by digest in `ci.yml` and `release.yml`. A tool or Rust version change is made there; moving to a newer image is a pull request here that changes the digest, so its checks run in the new image. `tools/setup.sh` links the image's toolchain as `vendor/` when `DOS_TOOLCHAIN` is set; developers' machines are unchanged.
- **GitHub tooling:** prefer the `gh` CLI for repository, pull request, label, review and Actions work.

## Local Tooling

The stack is **Rust** (pinned in `rust-toolchain.toml`, built with `RUSTC_BOOTSTRAP=1` for `-Zbuild-std` and `-Zjson-target-spec` on the stable toolchain), **C** (DJGPP's cross-compiler), **DOSBox Staging** for headless runs, **Bun** for the TypeScript exporters, and a little Python for `tools/coff2go32.py`. `tools/setup.sh` prepares a clone (it checks for the tools, installs the Rust pieces, runs `tools/fetch-tools.sh`, which downloads DJGPP, DOSBox Staging and CWSDPMI into `vendor/` with checksums, and checks out the main game at `FIZZ_GAME`, by default `../../liminal-hq/lieutenant-fizz` from the repository). Do not commit `vendor/`. Do not install system packages to get a build working without asking.

```bash
tools/setup.sh                       # check the tools, install the Rust pieces, fetch the DOS toolchain, clone the main game if missing
tools/check.sh [--quick]             # everything the CI runs
tools/clean.sh [--all]               # remove build outputs (--all also vendor/ and node_modules)
m2/build.sh                          # build the simulation for DOS and check it against the web build's wasm (PASS: 800 checkpoints)
m4/build.sh [--no-run] [level] [ticks]  # build the playable FIZZ.EXE (and run it headless)
tools/sync-sim.sh [--check]          # regenerate (or check) m2/sim and m2/game from the main game plus m2/patches
tools/dosrun.sh <dir> "<command>"    # run a DOS program headless in DOSBox
tools/make-dist.sh                   # pack dist/fizz-dos-demo.zip
tools/make-release.sh <version>      # build a release's files into dist/release/: floppy image, zips, browser bundle, SHA256SUMS
tools/make-site.sh                   # build the GitHub Pages bundle in site/
tools/make-share.sh                  # build the link-preview picture site/img/share.png
```

Treat the scripts as the source of truth for exact commands. Use `$TMPDIR`-style scratch space (not `/tmp`) when several jobs share a machine. Mute game audio before any browser check.

## Code Conventions

- **Rust (`m2/dos-rt`, `m2/sim`, `m2/game`):** `no_std` with `alloc`; no floating-point maths from `std` (use the `fmath` traits over `libm`); no `unsafe` without a one-line justification of what makes it sound. `m2/sim` and `m2/game` are the main game's crates plus the patches in `m2/patches`; regenerate them with `tools/sync-sim.sh`, and after an intended edit run `tools/sync-sim.sh --write-patches` so the patch and the copies agree (`tools/check.sh` fails when they do not).
- **Determinism:** the simulation must stay bit-for-bit identical to the web build's. The x87 precision control is set to 53 bits before the first tick; never change the order of floating-point operations in the simulation to chase speed.
- **C (`m4/c`):** C99 for DJGPP, one concern per file, volatile for anything an interrupt handler touches, no `malloc` in an interrupt handler.
- **DOS limits:** file names are 8.3; the keyboard and timer are hooked directly (IRQ 1 and IRQ 0), so every handler must restore the original vector on exit.
- **Rendering:** the frame is a 64,000-byte buffer; palette index is `level * 16 + colour`. Keep per-pixel work free of floating point (the x87 is slow, and DOSBox's emulation of it slower).
- **Assets:** `m3/assets/sprites.bin`, `m4/assets/sound.bin` and `m2/dos-rt/src/ui_data.rs` are generated by the exporters (`m3/export-sprites.ts`, `m4/export-sound.ts`, `m4/export-ui.ts`) from the main game and, for sound, the converters in `m4/sound`; `m4/assets/titles.bin` is generated by `m4/export-titles.ts` from the opening titles' oracle (`m4/titles/mock.ts`) and the committed sheet, font and mark, without the main game. Regenerate them, do not edit by hand. The generated files are committed, so a build needs neither Bun nor the main game.

## Testing

- Verify relevant changes before considering work complete, and state exactly what was and was not run.
- A change to the simulation or the drawing code keeps `m2/build.sh` at `PASS: 800 checkpoints identical to the web build's wasm` and keeps the frame checksums of `m5/bench` unless the picture is meant to change.
- `tools/check.sh` is the pre-PR gate and what the CI runs: licence headers, the simulation copies, determinism against the web build, the DOS build, the key map, and the bench's frame checksums (`m5/bench/expected-checksums.txt`) and pause-menu test.
- Speed work is measured, not guessed: the instruction counts from `m5/bench` first, then DOSBox at a fixed cycle count (`cycles=fixed 26800` for a 486DX2-66).
- Fix a bug with a check that fails before the fix.
- Say plainly what could not be verified: nothing here has run on real hardware, and audio cannot be checked headless.

## Documentation

- Update the relevant document in the same branch when behaviour changes: `docs/STATUS.md` for what is and is not ported, `docs/ARCHITECTURE.md` for the layers and build, `README.md` for how to play and build, and `dist/fizz-dos-demo/README.TXT` for the player-facing notes shipped with the zip.
- Prefer stable reference docs over burying decisions in code comments or PR threads.
- No hard wrapping: see [Markdown Formatting](#markdown-formatting).

## Repository Layout

A Cargo workspace for the native checks, with the DOS crates in their own workspace because they need a custom target:

- `m0/` — native determinism harness: runs the simulation and hashes its state (`m0/hash.ts` does the same through the web build's wasm)
- `m2/` — the simulation as a DOS program: `sim` and `game` (the main game's crates plus `patches/`), `dos-rt` (the DOS runtime: drawing, screens, saves, sound hooks) and `dos.json` (the custom i486 target)
- `m3/` — sprite export, the sprite census and the screenshot converter
- `m4/` — the playable build: the C layer (`c/`), the exporters, the sound converters (`sound/`), `build.sh`
- `m5/` — `bench`, a native 32-bit build that counts instructions per frame by stage and checks frame checksums
- `.github/` — the workflows (`ci.yml`, `pages.yml`, `release.yml`)
- `tools/` — setup, check, clean, fetch, run, pack and convert scripts, and the DOSBox configuration
- `dist/` — the demo folder (`fizz-dos-demo`) that `tools/make-dist.sh` zips
- `site/` — the GitHub Pages page and the js-dos bundle
- `docs/` — status, architecture and the milestone notes
- `assets/` — the README hero banner
- `attic/` — superseded code from the first milestones, not built or tested (see its README)

## Licence and Copyright

This project is dual-licensed under Apache-2.0 OR MIT (`LICENSE-APACHE`, `LICENSE-MIT`), as the main game is.

**REQUIREMENT:** New and substantially rewritten source files (`.rs`, `.c`, `.h`, `.ts`, `.js`, `.py`) MUST begin with a licence and copyright header:

```
// Brief one-line summary of what this file does.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT
```

For GitHub Actions workflow YAML, shell scripts and Python, use the same header with `#` comments; for C, `/* */` comments. Do not add headers to markdown, JSON, lockfiles or other generated or config-only files; the patched copies of the main game's crates keep the headers they came with. Third-party material is listed in `NOTICE`: CWSDPMI ships beside the game with its own licence, and js-dos (GPL-2.0) is served from `site/vendor/js-dos/` with its licence beside it. `tools/check-headers.sh` checks the headers; it skips the patched copies of the main game's crates, the sound converters in `m4/sound` (which keep their own headers), the generated `ui_data.rs`, the vendored js-dos and `attic/`.

## Agent Behaviour

- Read the relevant `docs/` section before implementing, and follow it.
- Make the change that was asked for; avoid speculative scaffolding and unrelated refactors.
- Do not push, merge, force-push, or alter shared settings without explicit instruction.
- State uncertainty and unverified items plainly rather than implying success.
- Stay within your assigned files when several agents work in the same checkout.
