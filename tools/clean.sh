#!/bin/bash
# Removes build outputs to free disk space: the Cargo target folders, the DOS objects and executables, and the run output. With
# --all it also removes vendor/ (the downloaded DOS toolchain, about 440 MB; tools/setup.sh brings it back) and node_modules.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# usage: tools/clean.sh [--all]
set -euo pipefail
cd "$(dirname "$0")/.."
size() { du -sh "$@" 2>/dev/null | cut -f1; }
before="$(du -sk . 2>/dev/null | cut -f1)"
rm -rf target target-tests m2/target m5/bench/target attic/*/rust/target attic/*/target
rm -f m2/*.a m2/c/*.exe m2/c/OUT.TXT m4/c/*.exe m4/c/OUT.TXT m4/c/FRAME.RAW m4/c/PAL.RAW m4/c/*.SAV m4/c/FIZZ.CFG
rm -rf out
if [ "${1:-}" = "--all" ]; then
  rm -rf vendor node_modules
fi
after="$(du -sk . 2>/dev/null | cut -f1)"
echo "freed about $(( (before - after) / 1024 )) MB; the repository folder is now $(size .)"
