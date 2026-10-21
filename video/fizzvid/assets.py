# The game's own material for the video: the DOS palette, footage from the real renderer, the pixel font, the sprites and the
# OPL2 music data.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT

import functools
import re
import struct
from pathlib import Path

import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[2]

# The 16 EGA colours as 6-bit VGA channels, in the sprite DSL's order (as m4/c/main.c has them).
EGA6 = [(0, 0, 0), (0, 0, 42), (0, 42, 0), (0, 42, 42), (42, 0, 0), (42, 0, 42), (42, 21, 0), (42, 42, 42),
        (21, 21, 21), (21, 21, 63), (21, 63, 21), (21, 63, 63), (63, 21, 21), (63, 21, 63), (63, 63, 21), (63, 63, 63)]


def palette6():
    """The game's palette as m4/c/main.c builds it: byte = level * 16 + colour, level 0 full, 1 black to 14 full."""
    pal = np.zeros((256, 3), np.uint8)
    for lvl in range(15):
        for c in range(16):
            for k in range(3):
                v = EGA6[c][k]
                pal[lvl * 16 + c, k] = v if lvl == 0 else (v * (lvl - 1) + 6) // 13
    return pal


PAL6 = palette6()


def to8(p6):
    """6-bit VGA channels to 8-bit, as a VGA DAC shows them."""
    return (p6.astype(np.uint16) * 255 // 63).astype(np.uint8)


def ega_rgb(c):
    """EGA colour c (0..15) as 8-bit RGB."""
    return tuple(int(v) * 255 // 63 for v in EGA6[c])


def frame_rgb(raw):
    """A footage frame (64,000 palette indices and the clear colour's three 6-bit channels) as a 320 x 200 RGB array."""
    idx = np.frombuffer(raw, np.uint8, 64000).reshape(200, 320)
    pal = PAL6.copy()
    pal[255] = np.frombuffer(raw[64000:64003], np.uint8)
    return to8(pal)[idx]


class Clip:
    """A folder of footage frames written by the bench (FOOTAGE=<dir>), 30 a second."""

    def __init__(self, folder):
        self.files = sorted(Path(folder).glob('*.raw'))
        if not self.files:
            raise FileNotFoundError(f'no footage in {folder}')

    def __len__(self):
        return len(self.files)

    def frame(self, t, start=0.0, loop=False, span=None):
        """The RGB frame at t seconds into the clip (from `start` seconds), held at the last frame or looped (over the first
        `span` seconds, when given)."""
        n = min(len(self.files), int(span * 30)) if span else len(self.files)
        i = int((start + max(t, 0.0)) * 30)
        i = i % n if loop else min(i, n - 1)
        return _frame_cached(str(self.files[i]))


@functools.lru_cache(maxsize=64)
def _frame_cached(path):
    return frame_rgb(Path(path).read_bytes())


@functools.lru_cache(maxsize=1)
def font():
    """The game's pixel font from m2/dos-rt/src/ui_data.rs: {char: (width, rows)}, the leftmost pixel in the highest bit."""
    text = (ROOT / 'm2/dos-rt/src/ui_data.rs').read_text()
    glyphs = {}
    for cp, w, rows in re.findall(r'Glyph \{ cp: (\d+), w: (\d+), rows: &\[([0-9, ]*)\] \}', text):
        glyphs[chr(int(cp))] = (int(w), [int(r) for r in rows.split(',') if r.strip()])
    return glyphs


@functools.lru_cache(maxsize=1)
def sprite_names():
    src = (ROOT / 'm2/game/src/sprites.rs').read_text()
    return re.findall(r'\w+\s*=\s*"([^"]+)"', src[src.index('sprites! {'):])


@functools.lru_cache(maxsize=None)
def sprite(name):
    """A sprite from m3/assets/sprites.bin as an RGBA image at full brightness."""
    d = (ROOT / 'm3/assets/sprites.bin').read_bytes()
    i = sprite_names().index(name)
    o = struct.unpack_from('<I', d, 2 + i * 4)[0]
    w, h = struct.unpack_from('<HH', d, o)
    pix = np.frombuffer(d, np.uint8, w * h, o + 6).reshape(h, w)
    rgba = np.zeros((h, w, 4), np.uint8)
    pal = to8(PAL6)
    rgba[..., :3] = pal[224 + (pix & 15)]
    rgba[..., 3] = np.where(pix == 255, 0, 255)
    return Image.fromarray(rgba, 'RGBA')


@functools.lru_cache(maxsize=1)
def song_ids():
    src = (ROOT / 'm4/sound/fizz-songs.ts').read_text()
    return re.findall(r"^\t\tid: '([a-z]+)'", src, re.M)


def song_index(name):
    return song_ids().index(name)


@functools.lru_cache(maxsize=None)
def channel_activity(song, fps=30, seconds=120):
    """Which OPL2 voices are sounding in each video frame of a song, from its IMF data in m4/assets/sound.bin: an array
    (frames, 14) of 0..1, melodic channels 0..8 then the five rhythm drums (bass, snare, tom, cymbal, hi-hat). The song loops."""
    d = (ROOT / 'm4/assets/sound.bin').read_bytes()
    off = struct.unpack_from('<I', d, 9 + song * 8)[0]
    n = struct.unpack_from('<H', d, off)[0]
    recs = d[off + 2:off + 2 + n]
    events = []  # (tick, reg, val)
    tick = 0
    for j in range(0, len(recs) - 3, 4):
        reg, val, delay = recs[j], recs[j + 1], struct.unpack_from('<H', recs, j + 2)[0]
        events.append((tick, reg, val))
        tick += delay
    length = max(tick, 1)
    frames = fps * seconds
    out = np.zeros((frames, 14), np.float32)
    on = [False] * 9
    drums = 0
    level = np.zeros(14, np.float32)
    ei = 0
    loop = 0
    for f in range(frames):
        now = f * 560 // fps
        while True:
            if ei >= len(events):
                ei = 0
                loop += length
            t, reg, val = events[ei]
            if t + loop > now:
                break
            if 0xB0 <= reg <= 0xB8:
                ch = reg - 0xB0
                if val & 0x20 and not on[ch]:
                    level[ch] = 1.0
                on[ch] = bool(val & 0x20)
            elif reg == 0xBD:
                hits = val & 0x1F & ~drums
                for k in range(5):
                    if hits & (1 << (4 - k)):
                        level[9 + k] = 1.0
                drums = val & 0x1F
            ei += 1
        for ch in range(9):
            if on[ch]:
                level[ch] = max(level[ch] * 0.93, 0.45)
            else:
                level[ch] *= 0.75
        level[9:] *= 0.7
        out[f] = level
    return out
