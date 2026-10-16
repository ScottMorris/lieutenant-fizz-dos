#!/bin/bash
# Everything the CI runs: licence headers, the simulation copies against the main game, the determinism check against the web
# build, a build of the playable game, the key map, and the bench's frame checksums and menu test. Stops at the first failure.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# usage: tools/check.sh [--quick]     (--quick skips the DOS determinism run, which takes about a minute)
#
# Needs what tools/setup.sh installs. If a drawing or simulation change is meant to alter the picture, run
#   NOCOUNT=1 m5/bench/target/i586-unknown-linux-gnu/release/render-bench 300 | grep 'frame checksum' > m5/bench/expected-checksums.txt
# and say why in the commit.
set -euo pipefail
cd "$(dirname "$0")/.."
QUICK=0; [ "${1:-}" = "--quick" ] && QUICK=1
step() { printf '\n== %s\n' "$1"; }

step "Licence headers"
tools/check-headers.sh

step "Simulation copies match the main game plus the patches"
tools/sync-sim.sh --check

if [ "$QUICK" = 0 ]; then
  step "Determinism against the web build (m2/build.sh)"
  m2/build.sh | tee "${TMPDIR:-/tmp}/fizz-parity.txt" | tail -3
  grep -q '^PASS: 800 checkpoints' "${TMPDIR:-/tmp}/fizz-parity.txt"
fi

step "The playable build (m4/build.sh --no-run)"
m4/build.sh --no-run
ls -l m4/c/fizz.exe

step "Key map"
CC_BIN="$(mktemp "${TMPDIR:-/tmp}/keymap_test.XXXXXX")"
gcc -o "$CC_BIN" m4/c/keymap_test.c m4/c/keymap.c
"$CC_BIN"
rm -f "$CC_BIN"

step "Bench: frame checksums and the pause menu"
(cd m5/bench && cargo build --release --target i586-unknown-linux-gnu)
BENCH=m5/bench/target/i586-unknown-linux-gnu/release/render-bench
NOCOUNT=1 "$BENCH" 300 | grep 'frame checksum' > "${TMPDIR:-/tmp}/fizz-checksums.txt"
diff -u m5/bench/expected-checksums.txt "${TMPDIR:-/tmp}/fizz-checksums.txt"
echo "frame checksums match"
PAUSETEST=1 "$BENCH" | tee "${TMPDIR:-/tmp}/fizz-pause.txt"
grep -q '^control, a fresh Enter on the marker: mode 3' "${TMPDIR:-/tmp}/fizz-pause.txt"
grep -q '^pause, Resume with Enter held: mode 2' "${TMPDIR:-/tmp}/fizz-pause.txt"

printf '\nAll checks passed.\n'
