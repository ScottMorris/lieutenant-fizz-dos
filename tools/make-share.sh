#!/bin/bash
# Builds site/img/share.png, the 1200 x 630 picture that link previews show: the README banner over three game screens.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# Reads site/img/hero.svg and the screenshots that tools/make-site.sh copies into site/img. Needs rsvg-convert and ImageMagick.
# The pixels come out the same for the same ImageMagick; another version may differ in the PNG's bytes.

set -euo pipefail
cd "$(dirname "$0")/.."

command -v rsvg-convert >/dev/null || { echo "make-share.sh: rsvg-convert is needed (librsvg)" >&2; exit 1; }
if command -v magick >/dev/null; then IM=(magick); elif command -v convert >/dev/null; then IM=(convert); else
  echo "make-share.sh: ImageMagick is needed" >&2; exit 1
fi

IMG=site/img
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT

for f in hero.svg level0.png map240.png level1.png; do
  [ -f "$IMG/$f" ] || { echo "make-share.sh: $IMG/$f is missing (tools/make-site.sh copies the screenshots)" >&2; exit 1; }
done

rsvg-convert -w 1200 "$IMG/hero.svg" -o "$WORK/hero.png"
for f in level0 map240 level1; do
  "${IM[@]}" "$IMG/$f.png" -filter point -resize 384x240 "$WORK/$f.png"
done
# The banner is 1200 x 320 at the top; the three screens sit below it, 8 px in from each side with 16 px between them.
"${IM[@]}" -size 1200x630 xc:'#050507' \
  "$WORK/hero.png" -geometry +0+0 -composite \
  "$WORK/level0.png" -geometry +8+355 -composite \
  "$WORK/map240.png" -geometry +408+355 -composite \
  "$WORK/level1.png" -geometry +808+355 -composite \
  -strip "$IMG/share.png"
ls -l "$IMG/share.png"
