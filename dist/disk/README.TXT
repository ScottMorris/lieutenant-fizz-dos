LIEUTENANT FIZZ - EPISODE 1: THE COCOA CAPER - for MS-DOS
=========================================================

This disk holds the DOS port of Lieutenant Fizz, Episode 1. The game's own
simulation, written in Rust, runs as a 32-bit DOS program in VGA mode 13h
(320x200, 256 colours) with AdLib music and effects, or PC-speaker effects.

It is built for a 486 with an x87 maths unit, and needs a VGA and at
least 1 MB of free memory; with less than 10 MB the DOS extender pages to
disk, which is slower. It has been run in DOSBox only: nothing has been
tried on real hardware yet.

RUN IT
  From the disk:      A:
                      FIZZ
  From a hard disk:   copy everything on this disk to a folder, then run
                      FIZZ.EXE there.

  FIZZ.EXE starts with a system check (what it found and a speed test),
  then Enter goes on to the opening titles (any key skips them) and the
  title menu. If the machine lacks a VGA, an x87 or enough memory, the
  check says so and Esc leaves. Without a VGA the check is shown on the
  text screen; 'FIZZ TEXT' shows it that way on any machine. The sound
  is set on the Options screen, which offers music and the effects'
  chip only with an AdLib.
  CWSDPMI.EXE, the DOS extender, must stay beside FIZZ.EXE.

  FIZZ.EXE [text] [level|map] [nopreview] [adlib|speaker|nosound]
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
  Saves are AUTO.SAV (written each time you reach the map), QUICK.SAV (F5
  saves, F9 loads) and SLOT1.SAV to SLOT4.SAV (Pause, Save game). Options
  are in FIZZ.CFG. All of them are written beside FIZZ.EXE, so the disk
  must not be write-protected.

SOUND TEST
  TONETEST plays an A (440 Hz) on the AdLib and then on the PC speaker.

WHAT IS IN IT
  Episode 1 from start to finish: the opening titles, New game with its
  opening story (Esc skips it), the overworld, every level and boss, and
  the ending and credits. The titles play again if the title menu sits
  untouched for a few minutes. See STATUS.md in the source for what has
  and has not been checked.

OTHER FILES
  README.RTF   this text with formatting
  CHANGES.TXT  what changed, version by version
  LICENSE.TXT  the licences, and the notice for CWSDPMI
  CWSDPMI.DOC  the extender's own terms

Source, build scripts and releases:
  https://github.com/ScottMorris/lieutenant-fizz-dos
