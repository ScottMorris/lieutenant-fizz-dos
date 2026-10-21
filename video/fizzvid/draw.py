# Drawing helpers for the explainer video: fonts, the game's pixel font, a CRT monitor for footage, panels, easing and captions.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT

import functools
import math
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFilter, ImageFont

from . import assets

W, H = 1920, 1080

BG = (11, 16, 32)
BG2 = (22, 30, 54)
INK = (236, 241, 248)
SOFT = (150, 164, 186)
DIM = (88, 100, 124)
ORANGE = (249, 115, 22)
CYAN = (34, 211, 238)
MAGENTA = (232, 121, 249)
GREEN = (74, 222, 128)
YELLOW = (250, 204, 21)
RED = (248, 113, 113)
BLUE = (96, 165, 250)
PANEL = (20, 28, 48)
EDGE = (52, 66, 98)

FONT_DIRS = ['/usr/share/fonts/opentype/inter', '/usr/share/fonts/truetype/inter', '/usr/share/fonts/truetype/dejavu',
             '/usr/share/fonts', str(Path.home() / '.fonts'), '/Library/Fonts', 'C:/Windows/Fonts']
FACES = {
    'regular': ['Inter-Regular.otf', 'Inter-Regular.ttf', 'DejaVuSans.ttf'],
    'medium': ['Inter-Medium.otf', 'Inter-Medium.ttf', 'DejaVuSans.ttf'],
    'semibold': ['Inter-SemiBold.otf', 'Inter-SemiBold.ttf', 'DejaVuSans-Bold.ttf'],
    'bold': ['Inter-Bold.otf', 'Inter-Bold.ttf', 'DejaVuSans-Bold.ttf'],
    'black': ['InterDisplay-Black.otf', 'Inter-Black.otf', 'Inter-ExtraBold.otf', 'Inter-Bold.otf', 'DejaVuSans-Bold.ttf'],
    'mono': ['DejaVuSansMono.ttf', 'DejaVuSansMono-Bold.ttf'],
    'monobold': ['DejaVuSansMono-Bold.ttf', 'DejaVuSansMono.ttf'],
}


@functools.lru_cache(maxsize=None)
def _face_path(face):
    for name in FACES[face]:
        for d in FONT_DIRS:
            for p in Path(d).rglob(name) if Path(d).exists() else []:
                return str(p)
    raise FileNotFoundError(f'no font for {face}: install Inter and DejaVu (see video/README.md)')


@functools.lru_cache(maxsize=None)
def font(size, face='regular'):
    return ImageFont.truetype(_face_path(face), size)


def clamp(x, a=0.0, b=1.0):
    return max(a, min(b, x))


def ease(x):
    x = clamp(x)
    return x * x * (3 - 2 * x)


def ease_out(x):
    x = clamp(x)
    return 1 - (1 - x) ** 3


def ramp(t, start, length=0.5):
    """0 before `start`, easing to 1 over `length` seconds."""
    return ease((t - start) / length) if length > 0 else float(t >= start)


def fade(t, dur, inn=0.4, out=0.4):
    """1 in the middle of a scene, easing in at the start and out at the end."""
    return min(ramp(t, 0, inn), 1 - ramp(t, dur - out, out))


def mix(a, b, k):
    return tuple(int(round(x + (y - x) * k)) for x, y in zip(a, b))


def background(img, t=0.0, tint=BG2):
    """A dark gradient with a slow drifting grid, the house style."""
    arr = _bg_cached(tint)
    img.paste(Image.fromarray(arr), (0, 0))
    d = ImageDraw.Draw(img)
    off = int(t * 12) % 60
    col = mix(BG, tint, 0.55)
    for x in range(-60 + off, W, 60):
        d.line([(x, 0), (x, H)], fill=col, width=1)
    for y in range(-60 + off, H, 60):
        d.line([(0, y), (W, y)], fill=col, width=1)


@functools.lru_cache(maxsize=8)
def _bg_cached(tint):
    y = np.linspace(0, 1, H)[:, None, None]
    x = np.linspace(0, 1, W)[None, :, None]
    a = np.array(BG, np.float32)
    b = np.array(tint, np.float32)
    k = np.clip(0.75 * (1 - y) * (0.6 + 0.4 * x), 0, 1)
    return (a + (b - a) * k).astype(np.uint8)


class Layer:
    """A transparent layer drawn on and composited onto the frame at an opacity."""

    def __init__(self, img, alpha=1.0):
        self.img, self.alpha = img, alpha
        self.layer = Image.new('RGBA', img.size, (0, 0, 0, 0))
        self.d = ImageDraw.Draw(self.layer)

    def __enter__(self):
        return self

    def __exit__(self, *exc):
        if self.alpha <= 0:
            return
        layer = self.layer
        if self.alpha < 1:
            a = layer.getchannel('A').point(lambda v: int(v * self.alpha))
            layer.putalpha(a)
        self.img.paste(layer, (0, 0), layer)


def text(img, xy, s, size=40, face='regular', fill=INK, anchor='la', alpha=1.0, spacing=8, align='left'):
    if alpha <= 0:
        return
    with Layer(img, alpha) as L:
        L.d.multiline_text(xy, s, font=font(size, face), fill=fill, anchor=anchor, spacing=spacing, align=align)


def text_width(s, size, face='regular'):
    return font(size, face).getlength(s)


def wrap(s, width, size, face='regular'):
    words, lines, cur = s.split(), [], ''
    for w in words:
        trial = (cur + ' ' + w).strip()
        if text_width(trial, size, face) <= width or not cur:
            cur = trial
        else:
            lines.append(cur)
            cur = w
    if cur:
        lines.append(cur)
    return lines


def panel(img, box, fill=PANEL, edge=EDGE, radius=18, alpha=1.0, width=2):
    if alpha <= 0:
        return
    with Layer(img, alpha) as L:
        L.d.rounded_rectangle(box, radius=radius, fill=fill, outline=edge, width=width)


def pixel_text(s, colour=INK, k=4, shadow=None):
    """`s` in the game's own pixel font, each font pixel k screen pixels square, as an RGBA image."""
    glyphs = assets.font()
    width = sum((glyphs[c][0] + 1) if c in glyphs else 4 for c in s) + 1
    small = Image.new('RGBA', (max(width, 1), 11), (0, 0, 0, 0))
    px = small.load()
    x = 0
    for c in s:
        if c not in glyphs:
            x += 4
            continue
        w, rows = glyphs[c]
        for r, bits in enumerate(rows):
            for col in range(w):
                if bits >> (w - 1 - col) & 1:
                    px[x + col, r] = colour + (255,)
        x += w + 1
    big = small.resize((small.width * k, small.height * k), Image.NEAREST)
    if shadow:
        out = Image.new('RGBA', (big.width + k, big.height + k), (0, 0, 0, 0))
        sh = Image.new('RGBA', big.size, shadow + (255,))
        out.paste(sh, (k, k), big)
        out.paste(big, (0, 0), big)
        return out
    return big


def paste_alpha(img, piece, xy, alpha=1.0, anchor='la'):
    if alpha <= 0:
        return
    x, y = xy
    if anchor[0] == 'm':
        x -= piece.width // 2
    elif anchor[0] == 'r':
        x -= piece.width
    if anchor[1] == 'm':
        y -= piece.height // 2
    elif anchor[1] == 'b':
        y -= piece.height
    if piece.mode != 'RGBA':
        piece = piece.convert('RGBA')
    if alpha < 1:
        piece = piece.copy()
        piece.putalpha(piece.getchannel('A').point(lambda v: int(v * alpha)))
    img.paste(piece, (int(x), int(y)), piece)


def sprite(name, k=6):
    s = assets.sprite(name)
    return s.resize((s.width * k, s.height * k), Image.NEAREST)


@functools.lru_cache(maxsize=4)
def _scanlines(w, h):
    a = np.zeros((h, w), np.uint8)
    a[1::3, :] = 60
    return Image.fromarray(a, 'L')


def monitor(img, frame, box, alpha=1.0, bezel=True, label=None):
    """Footage (a 320 x 200 RGB array) shown in a CRT-like monitor filling `box` (x0, y0, x1, y1), pixels kept square."""
    if alpha <= 0:
        return
    x0, y0, x1, y1 = box
    w, h = x1 - x0, y1 - y0
    pic = Image.fromarray(frame).resize((w, h), Image.NEAREST)
    shade = Image.new('RGB', (w, h), (0, 0, 0))
    pic = Image.composite(shade, pic, _scanlines(w, h))
    with Layer(img, alpha) as L:
        if bezel:
            L.d.rounded_rectangle((x0 - 22, y0 - 22, x1 + 22, y1 + 22), radius=30, fill=(28, 30, 36), outline=(70, 74, 86), width=3)
            L.d.rounded_rectangle((x0 - 6, y0 - 6, x1 + 6, y1 + 6), radius=12, fill=(6, 8, 10))
        L.layer.paste(pic, (x0, y0))
        glow = Image.new('RGBA', (w, h), (255, 255, 255, 0))
        gd = ImageDraw.Draw(glow)
        gd.rectangle((0, 0, w, h // 3), fill=(255, 255, 255, 10))
        L.layer.alpha_composite(glow, (x0, y0))
        if label:
            L.d.text((x0, y1 + 34), label, font=font(26, 'semibold'), fill=SOFT, anchor='la')


def caption(img, s, alpha=1.0):
    """Subtitles at the bottom of the frame, on a dark band, two lines at most."""
    if not s or alpha <= 0:
        return
    lines = wrap(s, 1660, 37, 'medium')[:3]
    h = 50 * len(lines) + 26
    y0 = H - 34 - h
    with Layer(img, alpha) as L:
        L.d.rounded_rectangle((W // 2 - 880, y0, W // 2 + 880, H - 34), radius=16, fill=(5, 8, 16, 205))
        for i, line in enumerate(lines):
            L.d.text((W // 2, y0 + 15 + i * 50), line, font=font(37, 'medium'), fill=INK, anchor='ma')


def title(img, s, sub=None, t=1.0, y=90, colour=INK):
    """A scene heading at the top left that slides in."""
    a = ramp(t, 0.1, 0.5)
    dx = int((1 - a) * -40)
    text(img, (110 + dx, y), s, 64, 'black', colour, alpha=a)
    if sub:
        text(img, (112 + dx, y + 84), sub, 32, 'medium', SOFT, alpha=ramp(t, 0.3, 0.5))


def chip(img, xy, s, colour, alpha=1.0, size=30):
    """A small rounded label."""
    if alpha <= 0:
        return
    f = font(size, 'semibold')
    w = f.getlength(s)
    x, y = xy
    with Layer(img, alpha) as L:
        L.d.rounded_rectangle((x, y, x + w + 36, y + size + 22), radius=(size + 22) // 2, fill=mix(BG, colour, 0.22), outline=colour, width=2)
        L.d.text((x + 18, y + 10), s, font=f, fill=colour)
    return w + 36


def arrow(img, a, b, colour=SOFT, width=4, alpha=1.0, head=16):
    if alpha <= 0:
        return
    with Layer(img, alpha) as L:
        L.d.line([a, b], fill=colour, width=width)
        ang = math.atan2(b[1] - a[1], b[0] - a[0])
        p1 = (b[0] - head * math.cos(ang - 0.45), b[1] - head * math.sin(ang - 0.45))
        p2 = (b[0] - head * math.cos(ang + 0.45), b[1] - head * math.sin(ang + 0.45))
        L.d.polygon([b, p1, p2], fill=colour)


def dot(img, xy, r, colour, alpha=1.0, glow=True):
    if alpha <= 0:
        return
    with Layer(img, alpha) as L:
        if glow:
            L.d.ellipse((xy[0] - r * 2.2, xy[1] - r * 2.2, xy[0] + r * 2.2, xy[1] + r * 2.2), fill=colour + (60,))
        L.d.ellipse((xy[0] - r, xy[1] - r, xy[0] + r, xy[1] + r), fill=colour)


def along(points, k):
    """The point k (0..1) of the way along a polyline."""
    segs = [(points[i], points[i + 1]) for i in range(len(points) - 1)]
    lens = [math.dist(a, b) for a, b in segs]
    total = sum(lens) or 1
    d = clamp(k) * total
    for (a, b), l in zip(segs, lens):
        if d <= l:
            f = d / l if l else 0
            return (a[0] + (b[0] - a[0]) * f, a[1] + (b[1] - a[1]) * f)
        d -= l
    return points[-1]


def soft_glow(img, box, colour, alpha=0.5, blur=30):
    """A blurred glow of `colour` behind a box."""
    if alpha <= 0:
        return
    g = Image.new('RGBA', img.size, (0, 0, 0, 0))
    ImageDraw.Draw(g).rounded_rectangle(box, radius=30, fill=colour + (int(255 * alpha),))
    g = g.filter(ImageFilter.GaussianBlur(blur))
    img.paste(g, (0, 0), g)
