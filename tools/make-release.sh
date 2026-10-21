#!/bin/bash
# Builds the files of a release into dist/release/: a 1.44 MB floppy image, the disk's contents as a zip, the DOSBox demo zip, the
# browser bundle and SHA256SUMS.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# usage: tools/make-release.sh <version, such as v0.1.0>
#
# Run m4/build.sh --no-run first (it writes m4/c/fizz.exe). Needs zip, mkfs.fat (dosfstools), mtools (mcopy, mattrib, mdir), python3 and the DJGPP
# toolchain in vendor/. File times in the image and the zips are SOURCE_DATE_EPOCH (default: the last commit's time), so the same
# tree gives the same files. tools/make-site.sh builds site/game.jsdos, which this copies.

set -euo pipefail
cd "$(dirname "$0")/.."

VERSION="${1:?usage: tools/make-release.sh <version, such as v0.1.0>}"
PLAIN="${VERSION#v}"
EPOCH="${SOURCE_DATE_EPOCH:-$(git log -1 --format=%ct)}"
OUT=dist/release
DISK="$OUT/disk"
NAME="fizz-dos-$VERSION"
# FAT volume labels are 11 characters at most; the label says which episode the disk holds.
LABEL="FIZZ EP1"
FLOPPY_BYTES=1474560

for tool in zip mkfs.fat mcopy mattrib mdir mtype python3 sha256sum; do
  command -v "$tool" >/dev/null || { echo "make-release.sh: $tool is needed" >&2; exit 1; }
done
[ -f m4/c/fizz.exe ] || { echo "make-release.sh: m4/c/fizz.exe is missing (run m4/build.sh --no-run)" >&2; exit 1; }
[ -f site/game.jsdos ] || { echo "make-release.sh: site/game.jsdos is missing (run tools/make-site.sh)" >&2; exit 1; }
[ -f vendor/cwsdpmi/bin/CWSDPMI.EXE ] || { echo "make-release.sh: the extender is missing (run tools/setup.sh)" >&2; exit 1; }
unzip -p site/game.jsdos FIZZ.EXE | cmp -s - m4/c/fizz.exe \
  || { echo "make-release.sh: site/game.jsdos holds a different FIZZ.EXE (run tools/make-site.sh)" >&2; exit 1; }

rm -rf "$OUT"
mkdir -p "$DISK"

# The disk: the game, its extender and the test tone, then the documents. Text goes in with DOS line ends and in ASCII.
cp m4/c/fizz.exe "$DISK/FIZZ.EXE"
cp vendor/cwsdpmi/bin/CWSDPMI.EXE "$DISK/CWSDPMI.EXE"
cp vendor/cwsdpmi/bin/cwsdpmi.doc "$DISK/CWSDPMI.DOC"
PATH="$PWD/vendor/djgpp/djgpp/bin:$PATH" i586-pc-msdosdjgpp-gcc -O2 -o "$DISK/TONETEST.EXE" tools/dos/tonetest.c
cp dist/disk/README.RTF "$DISK/README.RTF"
python3 -I - "$PLAIN" "$DISK" <<'PY'
import re
import sys

version, disk = sys.argv[1], sys.argv[2]
ASCII = {"—": "-", "–": "-", "’": "'", "‘": "'", "“": '"', "”": '"', "×": "x"}


def put(name, text):
    for k, v in ASCII.items():
        text = text.replace(k, v)
    text.encode("ascii")  # fails on anything the DOS code page would show wrongly
    with open(f"{disk}/{name}", "wb") as f:
        f.write(text.replace("\r\n", "\n").replace("\n", "\r\n").encode("ascii"))


for name in ("README.TXT", "COCOA.TXT"):
    put(name, open(f"dist/disk/{name}", encoding="utf-8").read())

# CHANGES.TXT: this version's section of CHANGELOG.md, with the markdown taken out.
log = open("CHANGELOG.md", encoding="utf-8").read()
m = re.search(rf"^## {re.escape(version)} .*?(?=^## |\Z)", log, re.S | re.M)
if not m:
    sys.exit(f"CHANGELOG.md has no section for {version}")
lines = []
for line in m.group(0).rstrip().splitlines():
    line = line.replace("`", "")
    heading = re.match(r"(#+) (.*)", line)
    if heading:
        title = heading.group(2).upper() if len(heading.group(1)) == 3 else heading.group(2)
        lines += ["", title, "=" * len(title) if len(heading.group(1)) == 2 else "-" * len(title), ""]
    else:
        lines.append(line)
import textwrap

wrapped = []
for line in lines:
    if line.startswith("- "):
        wrapped += textwrap.wrap(line, 76, subsequent_indent="  ")
    else:
        wrapped += textwrap.wrap(line, 76) or [""]
body = re.sub(r"\n{3,}", "\n\n", "\n".join(wrapped).strip("\n"))
put("CHANGES.TXT", "LIEUTENANT FIZZ FOR DOS - CHANGES\n\n" + body + "\n")

# LICENSE.TXT: the two licences the game is under, then the notice that names the third-party parts.
rule = "\n" + "=" * 78 + "\n"
parts = [("Apache License 2.0", "LICENSE-APACHE"), ("MIT License", "LICENSE-MIT"), ("Notice", "NOTICE")]
put("LICENSE.TXT", "Lieutenant Fizz for DOS is licensed under Apache-2.0 OR MIT, at your choice.\n" + "".join(
    f"{rule}{title}{rule}\n{open(path, encoding='utf-8').read().rstrip()}\n" for title, path in parts))
PY

# One time for every file, so the same tree gives the same image and zips.
find "$DISK" -type f -exec touch -d "@$EPOCH" {} +
FILES=(FIZZ.EXE CWSDPMI.EXE CWSDPMI.DOC TONETEST.EXE README.TXT README.RTF CHANGES.TXT LICENSE.TXT COCOA.TXT)

(cd "$DISK" && zip -q -X -D "../$NAME-disk.zip" "${FILES[@]}")

IMG="$OUT/$NAME-floppy.img"
# --invariant fixes the volume ID and the times that mkfs.fat would take from the clock (it reads SOURCE_DATE_EPOCH).
SOURCE_DATE_EPOCH="$EPOCH" mkfs.fat -C -F 12 -n "$LABEL" --invariant "$IMG" 1440 >/dev/null
(cd "$DISK" && mcopy -m -i "../$NAME-floppy.img" "${FILES[@]}" ::)
# The hidden file is for whoever lists the disk with `dir /a`.
mattrib -i "$IMG" +h +r ::COCOA.TXT

# The DOSBox demo: the static files in dist/fizz-dos-demo with the game and the extender (the zip tools/make-dist.sh packs, with the
# same times as the rest).
DEMO="$OUT/demo"
mkdir "$DEMO"
cp dist/fizz-dos-demo/{DOSBOX.CONF,RUN.BAT,run.sh,README.TXT} "$DEMO/"
cp "$DISK"/{FIZZ.EXE,CWSDPMI.EXE,CWSDPMI.DOC} "$DEMO/"
find "$DEMO" -type f -exec touch -d "@$EPOCH" {} +
(cd "$DEMO" && zip -q -X -D "../$NAME-dosbox.zip" DOSBOX.CONF RUN.BAT run.sh README.TXT FIZZ.EXE CWSDPMI.EXE CWSDPMI.DOC)
rm -rf "$DEMO"
cp site/game.jsdos "$OUT/$NAME-browser.jsdos"

# What the release must be: a 1.44 MB floppy that holds every file byte for byte.
[ "$(stat -c %s "$IMG")" -eq "$FLOPPY_BYTES" ] || { echo "make-release.sh: the image is not $FLOPPY_BYTES bytes" >&2; exit 1; }
for f in "${FILES[@]}"; do
  mtype -i "$IMG" "::$f" | cmp -s - "$DISK/$f" || { echo "make-release.sh: $f differs on the image" >&2; exit 1; }
done
mdir -i "$IMG" -a ::

(cd "$OUT" && sha256sum "$NAME-floppy.img" "$NAME-disk.zip" "$NAME-dosbox.zip" "$NAME-browser.jsdos" > SHA256SUMS)
rm -rf "$DISK"
ls -l "$OUT"
