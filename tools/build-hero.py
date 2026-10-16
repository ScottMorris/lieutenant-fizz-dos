#!/usr/bin/env python3
# Builds assets/hero.svg, the README banner: a pixel-art CRT showing the DOS title screen beside the project name.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# usage: python3 -I tools/build-hero.py > assets/hero.svg
import sys

# A 5 x 7 pixel font for the letters the banner's screen needs.
FONT = {
    "A": ["01110", "10001", "10001", "11111", "10001", "10001", "10001"],
    "B": ["11110", "10001", "10001", "11110", "10001", "10001", "11110"],
    "E": ["11111", "10000", "10000", "11110", "10000", "10000", "11111"],
    "F": ["11111", "10000", "10000", "11110", "10000", "10000", "10000"],
    "G": ["01110", "10001", "10000", "10111", "10001", "10001", "01111"],
    "I": ["11111", "00100", "00100", "00100", "00100", "00100", "11111"],
    "L": ["10000", "10000", "10000", "10000", "10000", "10000", "11111"],
    "M": ["10001", "11011", "10101", "10101", "10001", "10001", "10001"],
    "N": ["10001", "11001", "10101", "10101", "10011", "10001", "10001"],
    "O": ["01110", "10001", "10001", "10001", "10001", "10001", "01110"],
    "P": ["11110", "10001", "10001", "11110", "10000", "10000", "10000"],
    "S": ["01111", "10000", "10000", "01110", "00001", "00001", "11110"],
    "T": ["11111", "00100", "00100", "00100", "00100", "00100", "00100"],
    "U": ["10001", "10001", "10001", "10001", "10001", "10001", "01110"],
    "W": ["10001", "10001", "10001", "10101", "10101", "11011", "10001"],
    "Z": ["11111", "00001", "00010", "00100", "01000", "10000", "11111"],
    " ": ["00000"] * 7,
}


def text(s, x, y, k, fill):
    """Rects for `s` in the pixel font with `k` px per font pixel, from the top left (x, y)."""
    out = []
    for ch in s:
        g = FONT[ch]
        for r, row in enumerate(g):
            run = None
            for c, bit in enumerate(row + "0"):
                if bit == "1" and run is None:
                    run = c
                elif bit != "1" and run is not None:
                    out.append(f'<rect x="{x + run * k}" y="{y + r * k}" width="{(c - run) * k}" height="{k}" fill="{fill}"/>')
                    run = None
        x += 6 * k
    return "".join(out)


def width(s, k):
    return len(s) * 6 * k - k


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
title = "LIEUTENANT FIZZ"
k = 3
tw = width(title, k)
tx = (320 - tw) // 2
for dy, colour in ((2 * k, "#000000"), (k, "#ff5555"), (0, "#ffffff")):
    screen.append(text(title, SX + tx * K, SY + (30 + dy) * K, k * K, colour))
kick = "BEN BLAZE IN"
screen.append(text(kick, SX + ((320 - width(kick, 1)) // 2) * K, SY + 14 * K, K, "#ffff55"))
# The menu: the chosen row is a yellow plate with black ink.
screen.append(art(110, 112, 130, 12, "#ffff55"))
screen.append(text("NEW GAME", SX + 120 * K, SY + 114 * K, K, "#000000"))
screen.append(text("OPTIONS", SX + 120 * K, SY + 128 * K, K, "#aaaaaa"))
# The soda can that marks the chosen row, bobbing.
can = (
    f'<g class="bob"><rect x="{SX + 94 * K}" y="{SY + 109 * K}" width="{9 * K}" height="{16 * K}" fill="#aa0000"/>'
    f'<rect x="{SX + 95 * K}" y="{SY + 112 * K}" width="{7 * K}" height="{4 * K}" fill="#ffffff"/>'
    f'<rect x="{SX + 94 * K}" y="{SY + 109 * K}" width="{9 * K}" height="{K}" fill="#aaaaaa"/></g>'
)
screen = "".join(screen)

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
    <!-- House hero-text gradient: white into Afterglow orange and rose. -->
    <linearGradient id="brand" x1="0" y1="0" x2="1" y2="0">
      <stop offset="20%" stop-color="#ffffff"/>
      <stop offset="65%" stop-color="#ffaa40"/>
      <stop offset="100%" stop-color="#f43f5e"/>
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
  <text x="610" y="130" font-family="'JetBrains Mono', 'DejaVu Sans Mono', Menlo, Consolas, monospace" font-size="62" font-weight="700" fill="url(#brand)">Lieutenant Fizz</text>
  <text x="614" y="170" font-family="'JetBrains Mono', 'DejaVu Sans Mono', Menlo, Consolas, monospace" font-size="24" fill="#ffffff" fill-opacity="0.85">Episode 1 · The Cocoa Caper · for DOS</text>
  <g font-family="'JetBrains Mono', 'DejaVu Sans Mono', Menlo, Consolas, monospace" font-size="22">
    <text x="614" y="230" fill="#aaaaaa">C:\\&gt; FIZZ.EXE</text>
    <rect class="cursor" x="802" y="212" width="13" height="22" fill="#aaaaaa"/>
    <text x="614" y="268" fill="#55ffff" fill-opacity="0.9">VGA 320×200 · AdLib · 1.44 MB</text>
  </g>
</svg>
'''
sys.stdout.write(svg)
