# Converts a raw 320x200 frame and its palette, as written by the DOS build, to a PNG scaled 3 times.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT

import sys
from PIL import Image
frame = open(sys.argv[1], "rb").read()
pal = open(sys.argv[2], "rb").read()
im = Image.frombytes("P", (320, 200), frame)
flat = []
for i in range(256):
    r, g, b = pal[i * 3 : i * 3 + 3]
    flat += [min(255, r * 255 // 63), min(255, g * 255 // 63), min(255, b * 255 // 63)]
im.putpalette(flat)
im = im.convert("RGB").resize((960, 600), Image.NEAREST)
im.save(sys.argv[3])
print("wrote", sys.argv[3])
