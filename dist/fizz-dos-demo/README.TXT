LIEUTENANT FIZZ - EPISODE 1 - DOS demo (work in progress)
=========================================================

This is the real Episode 1 simulation, written in Rust, running as a 32-bit DOS program in VGA mode 13h
(320x200, 256 colours), with AdLib (OPL2) music and effects, or PC speaker effects if there is no AdLib, the game's own pixel font, a score/lives/Fizz HUD, map prompts, floating caption words, toasts and result cards. Everything is drawn from the game's own sprites. After the system check you reach the title (New game, Options, Quit; Continue and Load game once you have a save), then the overworld map; walk to a level marker and press Jump, Fire or Enter to go in.

The first thing you see is a system check in the manner of Keen's: what video, input, sound and maths hardware it found, a
speed test of the real renderer, and the sound it will use (A picks AdLib for everything, M the music on AdLib with the effects on the PC speaker, P the PC speaker alone, O off). Enter starts the
game. Add 'nopreview' on the command line to skip it.

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
  Esc              pause menu (and back, in a menu); Quit to title, then Quit, leaves the game

Notes
  - Saves are small files next to FIZZ.EXE: AUTO.SAV (written every time you reach the map) and SLOT1.SAV to SLOT4.SAV (Pause, Save game).
  - Options (music, effects, frame rate, detail, night, and the sound you chose at start-up) are kept in FIZZ.CFG, apart from the saves. Delete it to start fresh.
  - Options (title or pause menu): music, effects, which chip plays the effects, frame rate (auto, 60 or 30 Hz), detail (auto, high or low; low drops the backdrop layers on slow machines) and night.
  - The boss fight now starts with its dialogue: Enter finishes a line and moves on.
  - If it runs slowly with sound on, the OPL emulator is the cost: DOSBOX.CONF uses 'oplemu = compat' (light). Nuked is
    more exact but needs a fast machine.
  - Finishing a level, falling or running out of lives takes you back to the map after a moment (a card says what happened; Enter dismisses it).
  - Command line: 'nopreview' skips the system check; 'fps30', 'fps60', 'lowdetail', 'highdetail' override those options for one run.
  - 'FIZZ.EXE' starts at the title, 'FIZZ.EXE map' on the overworld and 'FIZZ.EXE 3' in level 3.
  - CWSDPMI.EXE must stay next to FIZZ.EXE (it is the DOS extender). Its licence is in CWSDPMI.DOC.
  - Source and build scripts: https://github.com/ScottMorris/lieutenant-fizz-dos
