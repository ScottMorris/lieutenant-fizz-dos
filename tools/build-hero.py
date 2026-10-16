#!/usr/bin/env python3
# Builds assets/hero.svg, the README banner: a pixel-art CRT showing the DOS title screen beside the project name.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# usage: python3 -I tools/build-hero.py > assets/hero.svg
import os
import re
import sys

DATA = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "m2", "dos-rt", "src", "ui_data.rs")

# The game's own pixel font, read from the generated interface data (see m4/export-ui.ts): each glyph has a width and rows from the
# top of a capital letter, the leftmost pixel in the highest bit; a letter advances by its width plus one pixel.
SPACE = 3
GLYPHS = {}
for cp, w, rows in re.findall(r"Glyph \{ cp: (\d+), w: (\d+), rows: &\[([^\]]*)\] \}", open(DATA).read()):
    GLYPHS[chr(int(cp))] = (int(w), [int(r) for r in rows.split(",") if r.strip()])


def advance(ch):
    return SPACE if ch == " " or ch not in GLYPHS else GLYPHS[ch][0] + 1


def text(s, x, y, k, fill):
    """Rects for `s` in the game's font with `k` px per font pixel, from the top left (x, y)."""
    out = []
    for ch in s:
        if ch in GLYPHS:
            w, rows = GLYPHS[ch]
            for r, bits in enumerate(rows):
                run = None
                for c in range(w + 1):
                    on = c < w and (bits >> (w - 1 - c)) & 1
                    if on and run is None:
                        run = c
                    elif not on and run is not None:
                        out.append(f'<rect x="{x + run * k}" y="{y + r * k}" width="{(c - run) * k}" height="{k}" fill="{fill}"/>')
                        run = None
        x += advance(ch) * k
    return "".join(out)


def width(s, k):
    return (sum(advance(ch) for ch in s) - 1) * k


def logo(s, x, y, k, colour="#ffffff", edge="#ff5555"):
    """The wordmark: `colour` letters, an `edge` line a font pixel under them, a black line under that."""
    return "".join(text(s, x, y + dy, k, c) for dy, c in ((2 * k, "#000000"), (k, edge), (0, colour)))


# The screen is 320 x 200 art pixels drawn at 1.2 px each.
SX, SY, K = 90, 38, 1.2
SW, SH = 320 * K, 200 * K


def art(x, y, w, h, fill):
    return f'<rect x="{SX + x * K}" y="{SY + y * K}" width="{w * K}" height="{h * K}" fill="{fill}"/>'


screen = []
screen.append(art(0, 0, 320, 200, "#000000"))
# Purple hills behind the ground, as in the title's attract loop.
hills = [(0, 120), (20, 100), (30, 118), (50, 96), (70, 120), (95, 104), (120, 122), (150, 98), (175, 120), (210, 100),
         (240, 124), (270, 106), (300, 122), (320, 110), (320, 150), (0, 150)]
pts = " ".join(f"{SX + a * K},{SY + b * K}" for a, b in hills)
screen.append(f'<polygon points="{pts}" fill="#7a1f7a"/>')
screen.append(art(0, 150, 320, 50, "#4a1a6a"))
screen.append(art(0, 149, 320, 2, "#2f9e2f"))
# The wordmark: white letters, a red edge a pixel under them, a black line under that.
title = "Lieutenant Fizz"
k = 3
tx = (320 - width(title, k)) // 2
screen.append(logo(title, SX + tx * K, SY + 30 * K, k * K))
kick = "Ben Blaze in"
screen.append(text(kick, SX + ((320 - width(kick, 1)) // 2) * K, SY + 14 * K, K, "#ffff55"))
# The menu: the chosen row is a yellow plate with black ink.
screen.append(art(110, 112, 130, 12, "#ffff55"))
screen.append(text("New game", SX + 120 * K, SY + 114 * K, K, "#000000"))
screen.append(text("Options", SX + 120 * K, SY + 128 * K, K, "#aaaaaa"))
# The soda can that marks the chosen row, bobbing.
can = (
    f'<g class="bob"><rect x="{SX + 94 * K}" y="{SY + 109 * K}" width="{9 * K}" height="{16 * K}" fill="#aa0000"/>'
    f'<rect x="{SX + 95 * K}" y="{SY + 112 * K}" width="{7 * K}" height="{4 * K}" fill="#ffffff"/>'
    f'<rect x="{SX + 94 * K}" y="{SY + 109 * K}" width="{9 * K}" height="{K}" fill="#aaaaaa"/></g>'
)
screen = "".join(screen)
# The name on the right, in the same lettering: as large as fits between the monitor and the edge.
BIG_X, BIG_W = 612, 560
BIG_K = BIG_W // (sum(advance(ch) for ch in 'Lieutenant Fizz') - 1)
big = logo('Lieutenant Fizz', BIG_X, 70, BIG_K)

svg = f'''<!-- Hero banner for the Lieutenant Fizz for DOS README. -->
<!-- -->
<!-- (c) Copyright 2026 Liminal HQ, Scott Morris -->
<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->
<svg width="1200" height="320" viewBox="0 0 1200 320" xmlns="http://www.w3.org/2000/svg" role="img" aria-labelledby="hero-title hero-desc">
  <title id="hero-title">Lieutenant Fizz for DOS</title>
  <desc id="hero-desc">Lieutenant Fizz: Episode 1, The Cocoa Caper, for DOS. A dark banner with a pixel-art CRT monitor showing the game's title screen — white lettering with a red edge over purple hills, a yellow menu plate and a bobbing soda can — beside the project name and a DOS prompt running FIZZ.EXE.</desc>

  <style>
    .bob {{ animation: bob 1.2s steps(1) infinite; }}
    @keyframes bob {{ 0% {{ transform: translateY(0); }} 50% {{ transform: translateY({-2 * K}px); }} }}
    .sweep {{ animation: sweep 4s linear infinite; }}
    @keyframes sweep {{ from {{ transform: translateY(0); }} to {{ transform: translateY({SH}px); }} }}
    .cursor {{ animation: blink 1s steps(1) infinite; }}
    @keyframes blink {{ 0% {{ opacity: 1; }} 50% {{ opacity: 0; }} }}
    .glow {{ animation: glow 3s ease-in-out infinite; }}
    @keyframes glow {{ 0%, 100% {{ opacity: 0.55; }} 50% {{ opacity: 0.9; }} }}
    @media (prefers-reduced-motion: reduce) {{
      .bob, .sweep, .cursor, .glow {{ animation: none; }}
      .sweep {{ display: none; }}
    }}
  </style>

  <defs>
    <linearGradient id="bg" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0%" stop-color="#050507"/>
      <stop offset="100%" stop-color="#15121f"/>
    </linearGradient>
    <radialGradient id="halo" cx="0.5" cy="0.5" r="0.5">
      <stop offset="0%" stop-color="#f43f5e" stop-opacity="0.35"/>
      <stop offset="100%" stop-color="#f43f5e" stop-opacity="0"/>
    </radialGradient>
    <clipPath id="screen"><rect x="{SX}" y="{SY}" width="{SW}" height="{SH}" rx="10"/></clipPath>
  </defs>

  <rect width="1200" height="320" fill="url(#bg)"/>
  <ellipse class="glow" cx="{SX + SW / 2}" cy="{SY + SH / 2}" rx="330" ry="170" fill="url(#halo)"/>

  <!-- The monitor. -->
  <rect x="{SX - 28}" y="{SY - 22}" width="{SW + 56}" height="{SH + 44}" rx="26" fill="#2a2733"/>
  <rect x="{SX - 18}" y="{SY - 14}" width="{SW + 36}" height="{SH + 28}" rx="20" fill="#16141c"/>
  <g clip-path="url(#screen)">
    {screen}
    {can}
    <rect class="sweep" x="{SX}" y="{SY - 24}" width="{SW}" height="24" fill="#ffffff" opacity="0.07"/>
  </g>

  <!-- The name and the prompt. -->
  {big}
  <text x="614" y="170" font-family="'JetBrains Mono', 'DejaVu Sans Mono', Menlo, Consolas, monospace" font-size="24" fill="#ffffff" fill-opacity="0.85">Episode 1 · The Cocoa Caper · for DOS</text>
  <g font-family="'JetBrains Mono', 'DejaVu Sans Mono', Menlo, Consolas, monospace" font-size="22">
    <text x="614" y="230" fill="#aaaaaa">C:\\&gt; FIZZ.EXE</text>
    <rect class="cursor" x="802" y="212" width="13" height="22" fill="#aaaaaa"/>
    <text x="614" y="268" fill="#55ffff" fill-opacity="0.9">VGA 320×200 · AdLib · 1.44 MB</text>
  </g>
</svg>
'''
sys.stdout.write(svg)
