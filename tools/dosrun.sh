#!/bin/bash
# usage: tools/dosrun.sh <dir with the .EXE and CWSDPMI.EXE> <command line...>
# Runs a DOS program headless in DOSBox Staging with the dir mounted as C:, output captured to <dir>/OUT.TXT.
set -euo pipefail
HERE="$(cd "$(dirname "$0")/.." && pwd)"
DIR="$(cd "$1" && pwd)"; shift
cp -n "$HERE/vendor/cwsdpmi/bin/CWSDPMI.EXE" "$DIR/" 2>/dev/null || true
rm -f "$DIR/OUT.TXT"
SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy timeout "${DOS_TIMEOUT:-60}" \
  "$HERE"/vendor/dosbox/*/dosbox --noprimaryconf --nolocalconf \
  -c "mount c $DIR" -c "c:" -c "$* > OUT.TXT" -c "exit" >/dev/null 2>&1 || echo "dosbox exit $?"
[ -f "$DIR/OUT.TXT" ] && cat "$DIR/OUT.TXT"
