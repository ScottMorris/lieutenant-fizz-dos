#!/bin/bash
# Milestone 3: Episode 1 on a VGA mode 13h screen, run headless in DOSBox with a screenshot written out.
# usage: m3/build.sh [level] [ticks]
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$PWD/vendor/djgpp/djgpp/bin:$PATH"
LEVEL="${1:-0}"; TICKS="${2:-240}"
[ -f m3/assets/sprites.bin ] || bun m3/export-sprites.ts > m3/assets/sprites.bin
(cd m2 && RUSTC_BOOTSTRAP=1 cargo build --release -p dos-rt)
tools/coff2go32.py m2/target/dos/release/libdos_rt.a m2/dos_rt_go32.a i586-pc-msdosdjgpp-ar
i586-pc-msdosdjgpp-gcc -O2 -o m3/c/fizz.exe m3/c/main.c m3/c/vga.c m3/c/kbd.c m3/c/keymap.c m3/c/chkstk.c m2/dos_rt_go32.a -lm
rm -f m3/c/FRAME.RAW m3/c/PAL.RAW
DOS_TIMEOUT=300 tools/dosrun.sh m3/c "FIZZ.EXE $LEVEL $TICKS shot nosync"
python3 -I m3/raw2png.py m3/c/FRAME.RAW m3/c/PAL.RAW "out/level$LEVEL.png"
