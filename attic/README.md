# Attic

Code that got the port to where it is and is no longer part of the build. None of it is built, run or tested by `tools/check.sh` or CI, and the scripts assume the folder layout they were written for (they `cd` to the repository root and name `m1/...` and `m3/...` paths), so they need those paths put back before they run again.

| Folder | What it was | Why it is here |
|---|---|---|
| `m1/` | A `no_std` Rust library with a hello-world function, linked into a DOS program by DJGPP and run in headless DOSBox: the first proof that Rust code could be a DOS `.EXE`. `build.sh` is the recipe; `rust/dos.json` is the first version of the custom i486 target. | The live build (`m2/`, `m4/`) does the same thing with the real simulation and has superseded it. `docs/MILESTONES.md` describes what it established. |
| `m3-first-frame/` | The first mode 13h program: a scripted walk through a level, drawn to the screen, with the first keyboard handler and key map. | `m4/c` holds the current versions of all of it, with sound, saves and the shell. |
| `starter/` | A text-mode title screen that handed over to the simulation and printed its draw list; its `Host` trait was the seam a DOS front end would fill in. It depended on the main game by relative path. | The DOS front end exists, so the seam is no longer a question. |
