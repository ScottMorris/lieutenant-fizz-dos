#!/bin/bash
# Milestone 5: Episode 1 with sound (OPL2 music and effects, PC-speaker fallback), run headless in DOSBox with a screenshot.
# usage: m4/build.sh [level] [ticks]      (m4/build.sh --no-run only builds)
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$PWD/vendor/djgpp/djgpp/bin:$PATH"
RUN=1; [ "${1:-}" = "--no-run" ] && { RUN=0; shift; }
LEVEL="${1:-0}"; TICKS="${2:-240}"
[ -f m3/assets/sprites.bin ] || bun m3/export-sprites.ts > m3/assets/sprites.bin
[ -f m4/assets/sound.bin ] || bun m4/export-sound.ts > m4/assets/sound.bin
(cd m2 && RUSTC_BOOTSTRAP=1 cargo build --release -p dos-rt)
tools/coff2go32.py m2/target/dos/release/libdos_rt.a m2/dos_rt_go32.a i586-pc-msdosdjgpp-ar
i586-pc-msdosdjgpp-gcc -O2 -fno-toplevel-reorder -o m4/c/fizz.exe m4/c/main.c m4/c/vga.c m4/c/kbd.c m4/c/keymap.c \
  m4/c/chkstk.c m4/c/sound.c m2/dos_rt_go32.a -lm
[ "$RUN" = 1 ] || exit 0
rm -f m4/c/FRAME.RAW m4/c/PAL.RAW
DOS_TIMEOUT=300 tools/dosrun.sh m4/c "FIZZ.EXE $LEVEL $TICKS shot nosync nosound"
python3 -I m3/raw2png.py m4/c/FRAME.RAW m4/c/PAL.RAW "out/sound-level$LEVEL.png"
