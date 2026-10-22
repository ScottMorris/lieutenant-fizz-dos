#!/bin/bash
# Builds the files of a release into dist/release/: a 1.44 MB floppy image, the disk's contents as a zip, the DOSBox demo zip, the
# browser bundle, a debug zip and SHA256SUMS. The game in all of them is the stripped FIZZ.EXE (tools/strip-exe.sh); the debug zip holds the
# unstripped one that m4/build.sh wrote, for reading a crash's traceback against.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# usage: tools/make-release.sh <version, such as v0.1.0>
#
# Run m4/build.sh --no-run first (it writes m4/c/fizz.exe). Needs zip, mkfs.fat (dosfstools), mtools (mcopy, mattrib, mdir), python3 and the DJGPP
# toolchain in vendor/. One time stamps everything: the files, the zips, the image's files and the image's own creation time (the volume
# label's) are all SOURCE_DATE_EPOCH (default: the last commit's time) read as UTC, so they agree with each other, and the same
# tree gives the same files. tools/make-site.sh builds site/game.jsdos, which this copies.

set -euo pipefail
cd "$(dirname "$0")/.."

VERSION="${1:?usage: tools/make-release.sh <version, such as v0.1.0>}"
PLAIN="${VERSION#v}"
EPOCH="${SOURCE_DATE_EPOCH:-$(git log -1 --format=%ct)}"
# mkfs.fat reads the time as UTC while mcopy and zip write file times as local time, so a builder's time zone would put the image's creation
# time hours from its files' and make the same tree give different files in different places.
export TZ=UTC
OUT=dist/release
DISK="$OUT/disk"
NAME="fizz-dos-$VERSION"
# FAT volume labels are 11 characters at most; the label says which episode the disk holds.
LABEL="FIZZ EP1"
FLOPPY_BYTES=1474560

# The volume serial number holds the game and the version: FE01 (Fizz, episode 01), then the major version as one hex digit, the
# minor as two and the patch as one, so 0.1.0 is FE01-0010 and 1.112.3 is FE01-1703. A suffix such as -test is ignored.
[[ "$PLAIN" =~ ^([0-9]+)\.([0-9]+)\.([0-9]+) ]] || { echo "make-release.sh: the version must look like v1.2.3" >&2; exit 1; }
MAJOR=${BASH_REMATCH[1]}; MINOR=${BASH_REMATCH[2]}; PATCH=${BASH_REMATCH[3]}
{ [ "$MAJOR" -lt 16 ] && [ "$MINOR" -lt 256 ] && [ "$PATCH" -lt 16 ]; } \
  || { echo "make-release.sh: the version does not fit the serial number (major and patch under 16, minor under 256)" >&2; exit 1; }
SERIAL=$(printf 'FE01%X%02X%X' "$MAJOR" "$MINOR" "$PATCH")

for tool in zip mkfs.fat mcopy mattrib mdir mtype python3 sha256sum; do
  command -v "$tool" >/dev/null || { echo "make-release.sh: $tool is needed" >&2; exit 1; }
done
[ -f m4/c/fizz.exe ] || { echo "make-release.sh: m4/c/fizz.exe is missing (run m4/build.sh --no-run)" >&2; exit 1; }
[ -f site/game.jsdos ] || { echo "make-release.sh: site/game.jsdos is missing (run tools/make-site.sh)" >&2; exit 1; }
[ -f vendor/cwsdpmi/bin/CWSDPMI.EXE ] || { echo "make-release.sh: the extender is missing (run tools/setup.sh)" >&2; exit 1; }

rm -rf "$OUT"
mkdir -p "$DISK"

# The disk: the game (stripped), its extender and the test tone, then the documents. Text goes in with DOS line ends and in ASCII.
tools/strip-exe.sh m4/c/fizz.exe "$DISK/FIZZ.EXE"
# The browser bundle, which tools/make-site.sh strips the same way, must hold exactly this game.
unzip -p site/game.jsdos FIZZ.EXE | cmp -s - "$DISK/FIZZ.EXE" \
  || { echo "make-release.sh: site/game.jsdos holds a different FIZZ.EXE (run tools/make-site.sh)" >&2; exit 1; }
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

# The unstripped game, which keeps the symbols a crash's traceback is read against (symify, from DJGPP, run on it).
DEBUG="$OUT/debug"
mkdir "$DEBUG"
cp m4/c/fizz.exe "$DEBUG/FIZZ.EXE"
touch -d "@$EPOCH" "$DEBUG/FIZZ.EXE"
(cd "$DEBUG" && zip -q -X -D "../$NAME-debug.zip" FIZZ.EXE)
rm -rf "$DEBUG"

IMG="$OUT/$NAME-floppy.img"
# The image's creation time is SOURCE_DATE_EPOCH, as the files' times are (mkfs.fat's --invariant would pin it to a constant of its own, in
# 2015); -i sets the serial number, which mkfs.fat would otherwise take from the time.
SOURCE_DATE_EPOCH="$EPOCH" mkfs.fat -C -F 12 -n "$LABEL" -i "$SERIAL" "$IMG" 1440 >/dev/null
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

# The image's creation time (the volume label's entry in the root directory) and every file's times are the one time, SOURCE_DATE_EPOCH.
python3 -I - "$IMG" "$EPOCH" <<'PY'
import datetime
import struct
import sys

image, epoch = open(sys.argv[1], "rb").read(), int(sys.argv[2])
bytes_per_sector, reserved, fats, root_entries, per_fat = (struct.unpack_from("<H", image, 11)[0], struct.unpack_from("<H", image, 14)[0],
                                                          image[16], struct.unpack_from("<H", image, 17)[0], struct.unpack_from("<H", image, 22)[0])
root = (reserved + fats * per_fat) * bytes_per_sector
# FAT keeps seconds in twos; the time is the epoch's, cut down to an even second.
want = datetime.datetime.fromtimestamp(epoch - epoch % 2, datetime.timezone.utc).replace(tzinfo=None)


def stamp(date, time):
    return datetime.datetime(1980 + (date >> 9), (date >> 5) & 15, date & 31, time >> 11, (time >> 5) & 63, (time & 31) * 2)


seen = 0
for i in range(root_entries):
    entry = image[root + i * 32 : root + i * 32 + 32]
    if entry[0] == 0:
        break
    if entry[0] == 0xE5:
        continue
    c_time, c_date, _, _, w_time, w_date = struct.unpack_from("<HHHHHH", entry, 14)
    for what, when in (("created", stamp(c_date, c_time)), ("written", stamp(w_date, w_time))):
        if when != want:
            sys.exit(f"make-release.sh: {entry[:11].decode('latin1').strip()} was {what} {when}, not {want} (SOURCE_DATE_EPOCH)")
    seen += 1
if seen < 10:
    sys.exit(f"make-release.sh: found {seen} entries in the image's root directory, not the label and nine files")
print(f"the image and its {seen - 1} files are all stamped {want} UTC")
PY

(cd "$OUT" && sha256sum "$NAME-floppy.img" "$NAME-disk.zip" "$NAME-dosbox.zip" "$NAME-browser.jsdos" "$NAME-debug.zip" > SHA256SUMS)
rm -rf "$DISK"
ls -l "$OUT"
