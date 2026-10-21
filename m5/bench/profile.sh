#!/bin/bash
# Instructions a frame for each bench scene under callgrind, for machines whose instruction counter the bench cannot open (a
# virtual machine, a CI runner), and optionally an estimate of i486 cycles from the instruction mix (cyc486.py).
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# usage: m5/bench/profile.sh [--cycles] [--top N] [scene ...]      (scenes: map day "cave)" night theatre; all by default)
#
# Each scene runs for 100 and for 200 frames; the difference over 100 is one frame (a sim step, the build and the paint) with
# the start-up taken out. Needs valgrind. Extra environment (LOW=1, TICKONLY=1) passes through to the bench.
set -euo pipefail
cd "$(dirname "$0")"
CYCLES=0; TOP=0; SCENES=()
while [ $# -gt 0 ]; do
  case "$1" in
    --cycles) CYCLES=1 ;;
    --top) TOP="$2"; shift ;;
    *) SCENES+=("$1") ;;
  esac
  shift
done
[ "${#SCENES[@]}" -gt 0 ] || SCENES=(map day "cave)" night theatre)
cargo build --release --target i586-unknown-linux-gnu >/dev/null 2>&1
BENCH="$PWD/target/i586-unknown-linux-gnu/release/render-bench"
OUT="$(mktemp -d "${TMPDIR:-/tmp}/fizz-profile.XXXXXX")"
trap 'rm -rf "$OUT"' EXIT
FLAGS=(--tool=callgrind)
[ "$CYCLES" = 1 ] && FLAGS+=(--dump-instr=yes --compress-pos=no --compress-strings=no)
for s in "${SCENES[@]}"; do
  tag="$(echo "$s" | tr -cd 'a-z0-9')"
  for n in 100 200; do
    SCENE="$s" NOCOUNT=1 valgrind "${FLAGS[@]}" --callgrind-out-file="$OUT/$tag.$n" "$BENCH" "$n" > "$OUT/$tag.$n.txt" 2> "$OUT/$tag.$n.err" &
  done
done
wait
for s in "${SCENES[@]}"; do
  tag="$(echo "$s" | tr -cd 'a-z0-9')"
  a=$(grep -o 'Collected : [0-9]*' "$OUT/$tag.100.err" | grep -o '[0-9]*$')
  b=$(grep -o 'Collected : [0-9]*' "$OUT/$tag.200.err" | grep -o '[0-9]*$')
  printf '%-8s %9d instructions a frame\n' "$tag" $(( (b - a) / 100 ))
  if [ "$CYCLES" = 1 ]; then
    python3 -I "$PWD/cyc486.py" "$OUT/$tag.100" "$OUT/$tag.200" 100 --top "$TOP" | sed 's/^/         /'
  fi
done
