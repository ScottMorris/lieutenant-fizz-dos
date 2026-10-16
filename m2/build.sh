#!/bin/bash
# Milestone 2: the real Episode 1 sim (a no_std copy under m2/sim and m2/game) as a DOS .EXE, checked against the web build.
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$PWD/vendor/djgpp/djgpp/bin:$PATH"
(cd m2 && RUSTC_BOOTSTRAP=1 cargo build --release -p dos-rt)
tools/coff2go32.py m2/target/dos/release/libdos_rt.a m2/dos_rt_go32.a i586-pc-msdosdjgpp-ar
i586-pc-msdosdjgpp-gcc -O2 -fno-toplevel-reorder -o m2/c/fizz.exe m2/c/main.c m3/c/chkstk.c m4/c/sound.c m4/c/save.c m2/dos_rt_go32.a -lm
mkdir -p out
[ -f out/wasm.txt ] || bun m0/hash.ts > out/wasm.txt
DOS_TIMEOUT=900 tools/dosrun.sh m2/c "FIZZ.EXE 3000 1" | tr -d '\r' > out/dos-53.lf
if diff -q out/wasm.txt out/dos-53.lf >/dev/null; then
  echo "PASS: $(wc -l < out/dos-53.lf) checkpoints identical to the web build's wasm"
else
  echo "FAIL: $(diff out/wasm.txt out/dos-53.lf | grep -c '^<') checkpoints differ"; exit 1
fi
