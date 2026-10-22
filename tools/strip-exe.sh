#!/bin/bash
# Writes a stripped copy of a DJGPP executable: the symbol table and the debugging sections taken out, and nothing the program runs changed.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# usage: tools/strip-exe.sh <FIZZ.EXE> <stripped copy>
#
# What a release and the browser bundle carry is this copy; m4/build.sh's m4/c/fizz.exe, which the checks and tools/dosrun.sh run, keeps its
# symbols. A crash's traceback (DJGPP prints addresses) is read against the unstripped file, with the toolchain's `symify`; the release
# keeps it in its debug zip. The script refuses unless the copy is smaller, the real-mode stub, the entry point and the bytes of the code
# and the data are exactly the original's, and no symbols are left.

set -euo pipefail
cd "$(dirname "$0")/.."
IN="${1:?usage: tools/strip-exe.sh <FIZZ.EXE> <stripped copy>}"
OUT="${2:?usage: tools/strip-exe.sh <FIZZ.EXE> <stripped copy>}"
BIN="$PWD/vendor/djgpp/djgpp/bin"
for tool in strip objcopy objdump nm; do
  [ -x "$BIN/i586-pc-msdosdjgpp-$tool" ] || { echo "strip-exe.sh: the DJGPP $tool is missing (run tools/setup.sh)" >&2; exit 1; }
done
TOOL() { "$BIN/i586-pc-msdosdjgpp-$1" "${@:2}"; }
TOOL objdump -f "$IN" 2>/dev/null | grep -q 'file format coff-go32-exe' || { echo "strip-exe.sh: $IN is not a DJGPP executable" >&2; exit 1; }

WORK="$(mktemp -d "${TMPDIR:-/tmp}/strip-exe.XXXXXX")"
trap 'rm -rf "$WORK"' EXIT
cp "$IN" "$WORK/stripped.exe"
# --strip-all takes out the symbols and the debugging and comment sections; --strip-debug alone would leave most of the weight.
TOOL strip --strip-all "$WORK/stripped.exe"

fail() { echo "strip-exe.sh: $1" >&2; exit 1; }
[ "$(stat -c %s "$WORK/stripped.exe")" -lt "$(stat -c %s "$IN")" ] || fail "the stripped copy is not smaller"
# The real-mode stub (the go32 loader) is the first 2048 bytes and must be untouched.
cmp -s -n 2048 "$IN" "$WORK/stripped.exe" || fail "the stub changed"
[ "$(TOOL objdump -f "$IN" | grep 'start address')" = "$(TOOL objdump -f "$WORK/stripped.exe" | grep 'start address')" ] || fail "the entry point changed"
for section in .text .data; do
  TOOL objcopy -O binary -j "$section" "$IN" "$WORK/a.bin"
  TOOL objcopy -O binary -j "$section" "$WORK/stripped.exe" "$WORK/b.bin"
  cmp -s "$WORK/a.bin" "$WORK/b.bin" || fail "$section changed"
done
[ "$(TOOL objdump -h "$IN" | awk '$2==".bss" {print $3}')" = "$(TOOL objdump -h "$WORK/stripped.exe" | awk '$2==".bss" {print $3}')" ] || fail ".bss changed"
TOOL nm "$WORK/stripped.exe" 2>&1 | grep -q 'no symbols' || fail "symbols are left"

cp "$WORK/stripped.exe" "$OUT"
echo "stripped $(basename "$IN"): $(stat -c %s "$IN") -> $(stat -c %s "$OUT") bytes"
