#!/bin/bash
# Packs dist/fizz-dos-demo.zip from the latest m3 build (run m3/build.sh first). The static files live in dist/fizz-dos-demo.
set -euo pipefail
cd "$(dirname "$0")/.."
D=dist/fizz-dos-demo
cp m3/c/fizz.exe "$D/FIZZ.EXE"
cp vendor/cwsdpmi/bin/CWSDPMI.EXE "$D/CWSDPMI.EXE"
cp vendor/cwsdpmi/bin/cwsdpmi.doc "$D/CWSDPMI.DOC"
rm -f dist/fizz-dos-demo.zip
(cd "$D" && zip -q ../fizz-dos-demo.zip DOSBOX.CONF RUN.BAT run.sh README.TXT FIZZ.EXE CWSDPMI.EXE CWSDPMI.DOC)
ls -la dist/fizz-dos-demo.zip
