LIEUTENANT FIZZ - EPISODE 1: THE COCOA CAPER - for MS-DOS
=========================================================

This disk holds the DOS port of Lieutenant Fizz, Episode 1. The game's own
simulation, written in Rust, runs as a 32-bit DOS program in VGA mode 13h
(320x200, 256 colours) with AdLib music and effects, or PC-speaker effects.

It is built for a 486 with an x87 maths unit. It has been run in DOSBox
only: nothing has been tried on real hardware yet.

RUN IT
  From the disk:      A:
                      FIZZ
  From a hard disk:   copy everything on this disk to a folder, then run
                      FIZZ.EXE there.

  FIZZ.EXE starts with a system check (what it found, a speed test and the
  sound it will use), then Enter goes on to the title. CWSDPMI.EXE, the DOS
  extender, must stay beside FIZZ.EXE.

  FIZZ.EXE [level|map] [nopreview] [adlib|speaker|nosound]
                      [fps30|fps60] [lowdetail|highdetail]
  'FIZZ 3' starts in level 3 and 'FIZZ map' on the overworld.

KEYS
  Arrows or WASD   move (Up and Down also climb and look)
  Ctrl or Z        jump
  Alt or X         pogo stick
  Space or C       fire
  Enter            confirm: enter a level marker, dismiss a card
  Esc              pause menu, and back in a menu
  Backspace        back to the overworld map
  PgUp / PgDn      previous or next level
  R, N, M, E       restart the level; night mode; music; effects

SAVES AND OPTIONS
  Saves are AUTO.SAV (written each time you reach the map) and SLOT1.SAV to
  SLOT4.SAV (Pause, Save game). Options are in FIZZ.CFG. All of them are
  written beside FIZZ.EXE, so the disk must not be write-protected.

SOUND TEST
  TONETEST plays an A (440 Hz) on the AdLib and then on the PC speaker.

WHAT IS MISSING
  The opening cinematic, the ending and credits, and the controls screen.
  See STATUS.md in the source for the full list.

OTHER FILES
  README.RTF   this text with formatting
  CHANGES.TXT  what is in this release
  LICENSE.TXT  the licences, and the notice for CWSDPMI
  CWSDPMI.DOC  the extender's own terms

Source, build scripts and releases:
  https://github.com/ScottMorris/lieutenant-fizz-dos
