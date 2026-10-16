LIEUTENANT FIZZ - EPISODE 1 - DOS demo (work in progress)
=========================================================

This is the real Episode 1 simulation, written in Rust, running as a 32-bit DOS program in VGA mode 13h
(320x200, 256 colours). Everything is drawn from the game's own sprites. There is no sound and no
menus yet: you start straight in a level.

Run it
  1. Install DOSBox Staging (https://www.dosbox-staging.org) or DOSBox-X.
  2. In this folder, run:  dosbox --conf DOSBOX.CONF      (RUN.BAT on Windows, ./run.sh on macOS and Linux)
     DOSBox mounts this folder as C: and starts FIZZ.EXE.
  Or by hand, from a DOSBox prompt:   mount c <this folder>    c:    FIZZ.EXE [level]

Keys
  Arrows or WASD   move (Up and Down also climb and look)
  Ctrl or Z        jump
  Alt or X         pogo stick
  Space or C       fire
  PgUp / PgDn      previous / next level (0-15)
  R                restart the level
  N                night mode (darker ambient light; lamps and glowing things stand out)
  P                pause
  Esc              quit

Notes
  - Dying restarts the level after a moment; finishing a level moves on to the next.
  - CWSDPMI.EXE must stay next to FIZZ.EXE (it is the DOS extender). Its licence is in CWSDPMI.DOC.
  - Source and build scripts: the fizz-dos side repository.
