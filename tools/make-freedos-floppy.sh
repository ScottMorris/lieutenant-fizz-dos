#!/bin/bash
# Builds a bootable FreeDOS copy of a release's floppy image: the game disk with FreeDOS's boot sector, kernel and command interpreter
# added, so the disk starts at an A:\> prompt on a machine (or emulator) with no DOS of its own.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# usage: tools/make-freedos-floppy.sh <game floppy image> <output image>
#
# The result is checked: 1.44 MB, a boot signature, the files, no saves, 64 KB free for saves and a boot in QEMU when it is installed.
#
# Needs curl, sha256sum, unzip, mtools (mcopy, mdir, mtype), dd and python3; qemu-system-i386 is used to boot the result when it is
# installed. The FreeDOS 1.4 Floppy Edition is downloaded once into vendor/freedos/ and checked against a pinned SHA-256. Only mtools and
# dd touch the images: no root and no loop mounts. The same inputs give the same image: the FreeDOS files keep their own times and the
# text files take SOURCE_DATE_EPOCH (default: the last commit's time).
#
# The two disks share the 1.44 MB FAT12 geometry, so FreeDOS's boot code runs on the game disk's file system: the boot sector keeps the
# game disk's bytes 3 to 61 (the BIOS parameter block, the serial number and the volume label) and takes the rest from FreeDOS's.

set -euo pipefail
cd "$(dirname "$0")/.."

GAME="${1:?usage: tools/make-freedos-floppy.sh <game floppy image> <output image>}"
OUT="${2:?usage: tools/make-freedos-floppy.sh <game floppy image> <output image>}"
EPOCH="${SOURCE_DATE_EPOCH:-$(git log -1 --format=%ct)}"
export TZ=UTC

FD_URL="https://download.freedos.org/1.4/FD14-FloppyEdition.zip"
FD_SHA256="45b1fa7c52dd996c3bfa5e352ffcd410781b952a6ad629f15a4c9ec4bbaefc5a"
FD_DIR="vendor/freedos"
FLOPPY_BYTES=1474560
# Space the image keeps free for what the game writes to the disk it runs from (AUTO.SAV, the slots, FIZZ.CFG), well above the 8,192
# bytes the game disk keeps (tools/make-release.sh): the image fails the build before it fills to the last cluster.
SAVE_RESERVE_BYTES=65536

die() { echo "make-freedos-floppy.sh: $*" >&2; exit 1; }
for tool in curl sha256sum unzip mcopy mdir mtype dd python3; do
  command -v "$tool" >/dev/null || die "$tool is needed"
done
[ "$(stat -c %s "$GAME")" -eq "$FLOPPY_BYTES" ] || die "$GAME is not a $FLOPPY_BYTES-byte floppy image"

# The FreeDOS boot floppy: downloaded once, checked every time, and unpacked into a fresh directory (only the 1.44 MB boot image).
mkdir -p "$FD_DIR"
ZIP="$FD_DIR/FD14-FloppyEdition.zip"
if [ ! -f "$ZIP" ] || ! echo "$FD_SHA256  $ZIP" | sha256sum -c --status; then
  curl -fsSL --retry 3 -o "$ZIP" "$FD_URL"
fi
echo "$FD_SHA256  $ZIP" | sha256sum -c --status || die "$ZIP does not match its pinned SHA-256 ($FD_SHA256)"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
unzip -q -j "$ZIP" 144m/x86BOOT.img -d "$WORK"
BOOT="$WORK/x86BOOT.img"
[ "$(stat -c %s "$BOOT")" -eq "$FLOPPY_BYTES" ] || die "FreeDOS's boot image is not $FLOPPY_BYTES bytes"

cp "$GAME" "$OUT"

# The boot sector: bytes 0 to 2 (the jump) and 62 to 511 (the boot code and the signature) from FreeDOS, the rest from the game disk.
{ dd if="$BOOT" bs=1 count=3 2>/dev/null
  dd if="$GAME" bs=1 skip=3 count=59 2>/dev/null
  dd if="$BOOT" bs=1 skip=62 count=450 2>/dev/null; } > "$WORK/sector0"
[ "$(stat -c %s "$WORK/sector0")" -eq 512 ] || die "the grafted boot sector is not 512 bytes"
dd if="$WORK/sector0" of="$OUT" bs=512 count=1 conv=notrunc 2>/dev/null

# The kernel and the command interpreter go in the root, where the boot code looks for KERNEL.SYS.
mcopy -m -i "$BOOT" ::KERNEL.SYS "$WORK/KERNEL.SYS"
mcopy -m -i "$BOOT" ::FREEDOS/BIN/COMMAND.COM "$WORK/COMMAND.COM"

# The start-up files, with DOS line ends. The game is not started for the player.
printf 'FILES=40\r\nBUFFERS=20\r\nLASTDRIVE=Z\r\n' > "$WORK/FDCONFIG.SYS"
printf '@ECHO OFF\r\nPATH=A:\\\r\nECHO.\r\nECHO Lieutenant Fizz: type FIZZ to play.\r\nECHO.\r\n' > "$WORK/AUTOEXEC.BAT"

# FREEDOS.TXT: what the disk bundles, where its source is and the licence it is under.
python3 -I - "$WORK/FREEDOS.TXT" <<'PY'
import sys

licence = open("site/vendor/js-dos/LICENSE-GPL-2.0.txt", encoding="utf-8").read().strip()
text = f"""FREEDOS ON THIS DISK
====================

This disk boots with parts of FreeDOS 1.4: the kernel (KERNEL.SYS), the
command interpreter (COMMAND.COM, FreeCOM) and the boot sector's code.
FreeDOS is free software under the GNU General Public License, version 2,
whose text follows. Lieutenant Fizz for DOS itself is under Apache-2.0 OR
MIT; see LICENSE.TXT.

FreeDOS project:      https://www.freedos.org/
FreeDOS 1.4 download: https://www.freedos.org/download/
This disk was made from the 1.4 Floppy Edition:
  https://download.freedos.org/1.4/FD14-FloppyEdition.zip
The kernel's and FreeCOM's source code is in the FreeDOS 1.4 distribution
and at https://github.com/FDOS/kernel and https://github.com/FDOS/freecom.

{"=" * 78}

{licence}
"""
for old, new in {"—": "-", "–": "-", "’": "'", "‘": "'", "“": '"', "”": '"'}.items():
    text = text.replace(old, new)
text.encode("ascii")
open(sys.argv[1], "wb").write(text.replace("\r\n", "\n").replace("\n", "\r\n").encode("ascii"))
PY
for f in FDCONFIG.SYS AUTOEXEC.BAT FREEDOS.TXT; do touch -d "@$EPOCH" "$WORK/$f"; done

mcopy -m -i "$OUT" "$WORK/KERNEL.SYS" "$WORK/COMMAND.COM" "$WORK/FDCONFIG.SYS" "$WORK/AUTOEXEC.BAT" "$WORK/FREEDOS.TXT" ::

# What the image must be: still 1.44 MB, bootable, holding the game disk's files and FreeDOS's, with the space the game's saves need.
[ "$(stat -c %s "$OUT")" -eq "$FLOPPY_BYTES" ] || die "the image is not $FLOPPY_BYTES bytes"
[ "$(dd if="$OUT" bs=1 skip=510 count=2 2>/dev/null | od -An -tx1 | tr -d ' \n')" = "55aa" ] || die "the boot sector has no 0x55AA signature"
for f in KERNEL.SYS COMMAND.COM FDCONFIG.SYS AUTOEXEC.BAT FREEDOS.TXT FIZZ.EXE CWSDPMI.EXE; do
  mdir -i "$OUT" "::$f" >/dev/null 2>&1 || die "$f is missing from the image's root"
done
mtype -i "$OUT" ::FIZZ.EXE | cmp -s - <(mtype -i "$GAME" ::FIZZ.EXE) || die "FIZZ.EXE differs from the game disk's"
# A run of the game writes AUTO.SAV, the slots, FIZZ.CFG and FIZZ.TMP beside itself; neither image may ship with any of them.
for image in "$GAME" "$OUT"; do
  if mdir -i "$image" :: | grep -Eq '^(AUTO|SLOT[0-9]|QUICK|FIZZ) +(SAV|CFG|TMP) '; then
    die "$image holds a save, options or temporary file; the images are built fresh from the build output"
  fi
done
FREE=$(mdir -i "$OUT" :: | grep 'bytes free' | tr -dc '0-9')
[ -n "$FREE" ] || die "could not read the image's free space"
[ "$FREE" -ge "$SAVE_RESERVE_BYTES" ] || die "the image has $FREE bytes free, under the $SAVE_RESERVE_BYTES kept for saves and options"
echo "the FreeDOS image has $FREE bytes free, $SAVE_RESERVE_BYTES of them kept for saves and options"

# Boot it headless and read the text screen, when QEMU is here: FreeDOS's banner and an A:\> prompt must be on it.
if command -v qemu-system-i386 >/dev/null; then
  python3 -I - "$OUT" "$WORK" <<'PY'
import json
import os
import socket
import subprocess
import sys
import time

image, work = sys.argv[1], sys.argv[2]
sock, dump = os.path.join(work, "qmp.sock"), os.path.join(work, "screen.bin")
qemu = subprocess.Popen(["qemu-system-i386", "-fda", image, "-boot", "a", "-display", "none", "-m", "16",
                         "-qmp", f"unix:{sock},server=on,wait=off"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
try:
    for _ in range(50):
        if os.path.exists(sock):
            break
        time.sleep(0.1)
    qmp = socket.socket(socket.AF_UNIX)
    qmp.connect(sock)
    stream = qmp.makefile("rw")
    stream.readline()

    def command(name, **arguments):
        stream.write(json.dumps({"execute": name, "arguments": arguments}) + "\n")
        stream.flush()
        while True:
            reply = json.loads(stream.readline())
            if "return" in reply or "error" in reply:
                return reply

    command("qmp_capabilities")
    time.sleep(8)
    reply = command("pmemsave", val=0xB8000, size=4000, filename=dump)
    if "error" in reply:
        sys.exit(f"make-freedos-floppy.sh: could not read the screen: {reply['error']}")
    cells = open(dump, "rb").read()
    rows = [cells[r * 160:(r + 1) * 160:2].decode("latin1").rstrip() for r in range(25)]
    screen = "\n".join(r for r in rows if r)
    if "FreeDOS" not in screen or "A:\\>" not in screen:
        sys.exit("make-freedos-floppy.sh: the image did not boot to an A:\\> prompt; the screen shows:\n" + screen)
    print("the FreeDOS image boots to an A:\\> prompt in QEMU")
finally:
    qemu.kill()
PY
else
  echo "qemu-system-i386 is not installed: the boot was not tried (the structure was checked)"
fi
