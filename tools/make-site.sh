#!/bin/bash
# Builds the GitHub Pages site for local use and for a release's files: packs FIZZ.EXE into site/game.jsdos (a zip with the DOSBox settings in
# .jsdos/) and copies the hero banner and the screenshots the page shows. The bundles are not committed: the live page serves a release's
# browser bundle (tools/stage-site.sh, run by .github/workflows/pages.yml), and the footer says "development build" here.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# usage: tools/make-site.sh [--no-build]      (js-dos itself is vendored by tools/vendor-jsdos.sh)
set -euo pipefail
cd "$(dirname "$0")/.."
[ "${1:-}" = "--no-build" ] || m4/build.sh --no-run
[ -f site/vendor/js-dos/js-dos.js ] || tools/vendor-jsdos.sh
STAGE="$(mktemp -d "${TMPDIR:-/tmp}/site.XXXXXX")"
trap 'rm -rf "$STAGE"' EXIT
mkdir "$STAGE/.jsdos"
cp m4/c/fizz.exe "$STAGE/FIZZ.EXE"
cp vendor/cwsdpmi/bin/CWSDPMI.EXE "$STAGE/CWSDPMI.EXE"
cp vendor/cwsdpmi/bin/cwsdpmi.doc "$STAGE/CWSDPMI.DOC"
# A test tone for the sound, to run from the DOS prompt after leaving the game (tools/dos/tonetest.c).
PATH="$PWD/vendor/djgpp/djgpp/bin:$PATH" i586-pc-msdosdjgpp-gcc -O2 -o "$STAGE/TONETEST.EXE" tools/dos/tonetest.c
cp site/dosbox.web.conf "$STAGE/.jsdos/dosbox.conf"
printf '{"version":"8.xx"}\n' > "$STAGE/.jsdos/jsdos.json"
rm -f site/game.jsdos site/game-default.jsdos site/game-compat.jsdos site/game-fast.jsdos
(cd "$STAGE" && zip -q -r -X "$OLDPWD/site/game.jsdos" .jsdos FIZZ.EXE TONETEST.EXE CWSDPMI.EXE CWSDPMI.DOC)
# The same bundle with another OPL emulator, for comparing by ear: the page loads one with ?opl=default or ?opl=compat.
for opl in default compat; do
  sed "s/^oplemu = fast/oplemu = $opl/" site/dosbox.web.conf > "$STAGE/.jsdos/dosbox.conf"
  (cd "$STAGE" && zip -q -r -X "$OLDPWD/site/game-$opl.jsdos" .jsdos FIZZ.EXE TONETEST.EXE CWSDPMI.EXE CWSDPMI.DOC)
done
mkdir -p site/img
cp assets/hero.svg site/img/hero.svg
for f in title-logo level0 level1 night1 map240 preview scr-pause scr-dialogue; do
  [ -f "out/$f.png" ] && cp "out/$f.png" "site/img/$f.png"
done
ls -l site/game.jsdos site/game-default.jsdos site/game-compat.jsdos
