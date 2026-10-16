LIEUTENANT FIZZ - EPISODE 1 - DOS demo (work in progress)
=========================================================

This is the real Episode 1 simulation, written in Rust, running as a 32-bit DOS program in VGA mode 13h
(320x200, 256 colours), with AdLib (OPL2) music and effects, or PC speaker effects if there is no AdLib, the game's own pixel font, a score/lives/Fizz HUD, map prompts, floating caption words, toasts and result cards. Everything is drawn from the game's own sprites. There is no
menus yet: you start on the overworld map, walk to a level marker and press Jump, Fire or Enter to go in.

Run it
  1. Install DOSBox Staging (https://www.dosbox-staging.org) or DOSBox-X.
  2. In this folder, run:  dosbox --conf DOSBOX.CONF      (RUN.BAT on Windows, ./run.sh on macOS and Linux)
     DOSBox mounts this folder as C: and starts FIZZ.EXE.
  Or by hand, from a DOSBox prompt:   mount c <this folder>    c:    FIZZ.EXE [level]
  Sound options after the level: adlib, speaker or nosound (the default finds an AdLib, else the PC speaker).

Keys
  Arrows or WASD   move (Up and Down also climb and look)
  Ctrl or Z        jump
  Alt or X         pogo stick
  Space or C       fire
  Enter            confirm: enter a level marker, dismiss a card
  Backspace        back to the overworld map
  PgUp / PgDn      previous / next level (0-15)
  R                restart the level
  N                night mode (darker ambient light; lamps and glowing things stand out)
  M                music on/off
  E                sound effects on/off
  P                pause
  Esc              quit

Notes
  - If it runs slowly with sound on, the OPL emulator is the cost: DOSBOX.CONF uses 'oplemu = compat' (light). Nuked is
    more exact but needs a fast machine.
  - Finishing a level, falling or running out of lives takes you back to the map after a moment (a card says what happened; Enter dismisses it).
  - 'FIZZ.EXE map' (the default) starts on the overworld, 'FIZZ.EXE 3' starts in level 3.
  - CWSDPMI.EXE must stay next to FIZZ.EXE (it is the DOS extender). Its licence is in CWSDPMI.DOC.
  - Source and build scripts: the fizz-dos side repository.
