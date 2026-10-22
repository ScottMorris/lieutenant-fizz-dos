#!/bin/bash
# Builds the GitHub Pages site for local use and for a release's files: packs FIZZ.EXE into site/game.jsdos (a zip with the DOSBox settings in
# .jsdos/) and copies the hero banner and the screenshots the page shows. The bundles are not committed: the live page serves a release's
# browser bundle (tools/stage-site.sh, run by .github/workflows/pages.yml), and the footer says "development build" here. Every file in a
# bundle is stamped SOURCE_DATE_EPOCH (default: the last commit's time) read as UTC, the time tools/make-release.sh stamps its files with.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# usage: tools/make-site.sh [--no-build]      (js-dos itself is vendored by tools/vendor-jsdos.sh)
set -euo pipefail
cd "$(dirname "$0")/.."
[ "${1:-}" = "--no-build" ] || m4/build.sh --no-run
[ -f site/vendor/js-dos/js-dos.js ] || tools/vendor-jsdos.sh
EPOCH="${SOURCE_DATE_EPOCH:-$(git log -1 --format=%ct)}"
export TZ=UTC # zip writes file times as local time; this keeps the bundle the same wherever it is built
STAGE="$(mktemp -d "${TMPDIR:-/tmp}/site.XXXXXX")"
trap 'rm -rf "$STAGE"' EXIT
mkdir "$STAGE/.jsdos"
tools/strip-exe.sh m4/c/fizz.exe "$STAGE/FIZZ.EXE" # the bundle carries the stripped game; m4/c/fizz.exe keeps its symbols
cp vendor/cwsdpmi/bin/CWSDPMI.EXE "$STAGE/CWSDPMI.EXE"
cp vendor/cwsdpmi/bin/cwsdpmi.doc "$STAGE/CWSDPMI.DOC"
# A test tone for the sound, to run from the DOS prompt after leaving the game (tools/dos/tonetest.c).
PATH="$PWD/vendor/djgpp/djgpp/bin:$PATH" i586-pc-msdosdjgpp-gcc -O2 -o "$STAGE/TONETEST.EXE" tools/dos/tonetest.c
cp site/dosbox.web.conf "$STAGE/.jsdos/dosbox.conf"
printf '{"version":"8.xx"}\n' > "$STAGE/.jsdos/jsdos.json"
rm -f site/game.jsdos site/game-default.jsdos site/game-compat.jsdos site/game-fast.jsdos
# One time for every file and folder, so the same tree gives the same bundle.
pack() {
  find "$STAGE" -mindepth 1 -exec touch -d "@$EPOCH" {} +
  (cd "$STAGE" && zip -q -r -X "$OLDPWD/$1" .jsdos FIZZ.EXE TONETEST.EXE CWSDPMI.EXE CWSDPMI.DOC)
}
pack site/game.jsdos
# The same bundle with another OPL emulator, for comparing by ear: the page loads one with ?opl=default or ?opl=compat.
for opl in default compat; do
  sed "s/^oplemu = fast/oplemu = $opl/" site/dosbox.web.conf > "$STAGE/.jsdos/dosbox.conf"
  pack "site/game-$opl.jsdos"
done
mkdir -p site/img
cp assets/hero.svg site/img/hero.svg
for f in title-logo level0 level1 night1 map240 preview scr-pause scr-dialogue; do
  [ -f "out/$f.png" ] && cp "out/$f.png" "site/img/$f.png"
done
ls -l site/game.jsdos site/game-default.jsdos site/game-compat.jsdos
