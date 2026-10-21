# The explainer's scenes: what is said in each (the captions, and the spoken form where text to speech needs help), which of the
# game's songs plays under it, and how it is drawn. A scene's `draw(img, t, s)` gets the time in seconds since the scene began
# and the scene's timing (`s.at[i]` is when line i starts, `s.dur` the scene's length, `s.song_t(t)` where the song is).
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT

import math
import re

import numpy as np
from PIL import Image, ImageDraw

from . import assets
from .draw import (BG, BLUE, CYAN, DIM, EDGE, GREEN, H, INK, MAGENTA, ORANGE, PANEL, RED, SOFT, W, YELLOW, Layer, along, arrow,
                   background, chip, clamp, dot, ease, ease_out, fade, font, mix, monitor, panel, paste_alpha, pixel_text,
                   ramp, sprite, text, title, wrap)


# How the voice should say words it would otherwise get wrong, applied to every spoken line.
PRONOUNCE = [
    (r'\bMS-DOS\b', 'M S doss'), (r'\bDOSBox\b', 'doss box'), (r'\bjs-dos\b', 'J S doss'), (r'\bDOS\b', 'doss'),
    (r'\bEGA\b', 'E G A'), (r'\bVGA\b', 'V G A'), (r'\bOPL2\b', 'O P L two'), (r'\bAdLib\b', 'Ad Lib'),
    (r'\bx87\b', 'x eighty-seven'), (r'\bWebAssembly\b', 'Web Assembly'), (r'\bPC\b', 'P C'),
]


def spoken_form(s):
    for pattern, say in PRONOUNCE:
        s = re.sub(pattern, say, s)
    return s


class Scene:
    def __init__(self, id, song, lines, draw, lead=0.6, tail=1.0, min_dur=0.0):
        self.id, self.song, self.draw, self.lead, self.tail, self.min_dur = id, song, draw, lead, tail, min_dur
        # Each line is its caption, or (caption, what the voice says); either way the voice's text goes through PRONOUNCE.
        self.lines = [(l, spoken_form(l)) if isinstance(l, str) else (l[0], spoken_form(l[1])) for l in lines]


CLIPS = {}


def clip(name):
    return CLIPS[name]


# ---------------------------------------------------------------------------------------------------------------- 1. intro

def draw_intro(img, t, s):
    if t < 2.9:
        # A DOS prompt, typed.
        img.paste((4, 6, 8), (0, 0, W, H))
        y, cursor = 120, None
        for prompt, cmd, start in [('C:\\>', 'CD FIZZ', 0.3), ('C:\\FIZZ>', 'FIZZ', 1.4)]:
            if t < start - 0.3:
                break
            typed = cmd[:clamp(int((t - start) / 0.12), 0, len(cmd))]
            text(img, (120, y), prompt + typed, 52, 'mono', (200, 205, 210))
            cursor = (120 + font(52, 'mono').getlength(prompt + typed), y)
            y += 70
        if cursor and int(t * 2.5) % 2 == 0:
            x, yy = cursor
            img.paste((200, 205, 210), (int(x) + 4, yy + 50, int(x) + 34, yy + 58))
        return
    background(img, t)
    box = (320, 40, 1600, 840)
    if t < s.lead:
        monitor(img, clip('preview').frame(0), box, alpha=ramp(t, 2.9, 0.25))
        return
    monitor(img, clip('title').frame(t - s.lead + 2.0, loop=True), box)
    if t > s.at[2]:
        for i, (label, col) in enumerate([('Rust', ORANGE), ('MS-DOS', CYAN), ('VGA 13h', MAGENTA), ('AdLib', GREEN)]):
            chip(img, (110, 120 + i * 80), label, col, ramp(t, s.at[2] + 0.3 * i, 0.5))


INTRO = Scene('intro', 'title', [
    ('This is Lieutenant Fizz, Episode 1: The Cocoa Caper.', 'This is Lieutenant Fizz, Episode One: The Cocoa Caper.'),
    ('A brand-new platformer, running as a real MS-DOS program.', 'A brand new platformer, running as a real M S DOS program.'),
    ("It's the same game as the web version, down to the last bit of its physics, built for a PC from the early nineties.",
     "It's the same game as the web version, down to the last bit of its physics, built for a P C from the early nineties."),
], draw_intro, lead=4.4, tail=0.8)


# ---------------------------------------------------------------------------------------------------------------- 2. story

def draw_story(img, t, s):
    background(img, t)
    title(img, 'The Cocoa Caper', 'Episode 1 of Lieutenant Fizz', t)
    monitor(img, clip('map').frame(t, loop=True), (110, 260, 1070, 860), alpha=ramp(t, 0.2, 0.5))
    cards = [
        (s.at[0], ['ben_stand', 'ben_run1', 'ben_run2'], 'Ben Blaze', 'that is you: Lieutenant Fizz', ORANGE),
        (s.at[1], ['billy'], 'Billy Blaze', 'the cousin who went missing', CYAN),
        (s.at[2], ['boss0', 'boss1'], "Mildred's Colossus", 'guarding the stolen cocoa', MAGENTA),
    ]
    for i, (start, names, name, sub, col) in enumerate(cards):
        a = ramp(t, start, 0.5)
        if a <= 0:
            continue
        y = 260 + i * 170
        dx = int((1 - a) * 60)
        panel(img, (1150 + dx, y, 1810 + dx, y + 150), alpha=a)
        frame = names[int(t * 6) % len(names)]
        sp = assets.sprite(frame)
        paste_alpha(img, sprite(frame, max(1, min(5, 124 // max(sp.width, sp.height)))), (1230 + dx, y + 75), a, 'mm')
        text(img, (1320 + dx, y + 34), name, 40, 'bold', col, alpha=a)
        text(img, (1320 + dx, y + 88), sub, 28, 'regular', SOFT, alpha=a)
    a = ramp(t, s.at[3], 0.5)
    for i, n in enumerate(['ben_pogo', 'saucer', 'soda', 'cheezie']):
        k = 4 if n != 'saucer' else 3
        bob = math.sin(t * 4 + i) * 8
        paste_alpha(img, sprite(n, k), (1220 + i * 160, 800 + bob), ramp(t, s.at[3] + i * 0.25, 0.4), 'mm')


STORY = Scene('story', 'map', [
    'You play Ben Blaze.',
    "His cousin Billy has vanished, and Billy's note points to Planet Zargoth.",
    'There, Mildred McMire is stealing every cocoa bean on Earth to build the biggest chocolate castle in the galaxy.',
    'Ben has a bicycle helmet, a flying saucer, a pogo stick and plenty of fizz.',
], draw_story)


# ---------------------------------------------------------------------------------------------------------------- 3. Keen

def keycap(img, x, y, w, label, pressed, colour, alpha=1.0):
    h = 120
    off = 10 if pressed else 0
    with Layer(img, alpha) as L:
        L.d.rounded_rectangle((x, y + 14, x + w, y + h + 14), radius=18, fill=(30, 34, 46))
        L.d.rounded_rectangle((x, y + off, x + w, y + h + off), radius=18, fill=mix((58, 64, 82), colour, 0.35 if pressed else 0.0),
                              outline=colour if pressed else (96, 104, 124), width=3)
        L.d.text((x + w / 2, y + off + h / 2), label, font=font(44, 'bold'), fill=INK, anchor='mm')


def draw_keen(img, t, s):
    background(img, t, (40, 24, 60))
    a = ramp(t, s.at[1], 0.6)
    text(img, (W // 2, 170), 'Billy Blaze', 110, 'black', CYAN, 'mm', alpha=ramp(t, s.at[0] + 0.5, 0.6))
    text(img, (W // 2, 290), '=', 80, 'bold', SOFT, 'mm', alpha=a)
    text(img, (W // 2, 400), 'Commander Keen', 110, 'black', YELLOW, 'mm', alpha=a)
    text(img, (W // 2, 490), "the hero of id Software's Commander Keen games", 34, 'medium', SOFT, 'mm', alpha=a)
    k0 = s.at[2]
    keys = [('Ctrl', 'jump', 360, 260, ORANGE), ('Alt', 'pogo', 680, 260, MAGENTA), ('Space', 'fire', 1000, 560, CYAN)]
    for i, (label, act, x, w, col) in enumerate(keys):
        ka = ramp(t, k0 + 0.3, 0.5)
        hit = k0 + 1.6 + i * 1.0
        pressed = hit <= t < hit + 0.45
        keycap(img, x, 600, w, label, pressed, col, ka)
        text(img, (x + w / 2, 790), act, 40, 'bold', col if t >= hit else DIM, 'mm', alpha=ka)
    if t > k0:
        n = 'ben_pogo' if int(t * 4) % 2 else 'ben_pogo2'
        y = 650 - abs(math.sin(t * 3.2)) * 110
        paste_alpha(img, sprite(n, 6), (1700, y), ramp(t, k0 + 0.5, 0.5), 'mm')


KEEN = Scene('keen', 'groove', [
    'If that sounds familiar, it should.',
    "Billy Blaze is Commander Keen's real name, and Lieutenant Fizz is a spiritual tie-in to the Keen games.",
    'Even the keys match Keen 4: Control to jump, Alt for the pogo stick and Space to fire.',
], draw_keen)


# ---------------------------------------------------------------------------------------------------------------- 4. the era

EVENTS = [
    (1990.70, 'Sep 1990', 'Dangerous Dave in\nCopyright Infringement', 'smooth scrolling on a PC', 1),
    (1990.95, 'Dec 1990', 'Commander Keen:\nInvasion of the Vorticons', 'shareware, through Apogee', 2),
    (1991.10, 'Feb 1991', 'id Software', 'founded', 2),
    (1991.95, 'Dec 1991', 'Keen 4 and 5:\nGoodbye, Galaxy!', 'AdLib music by Bobby Prince', 3),
    (1992.35, 'May 1992', 'Wolfenstein 3D', '', 4),
    (1993.95, 'Dec 1993', 'Doom', '', 4),
    (1996.47, 'Jun 1996', 'Quake', '', 4),
]


def draw_era(img, t, s):
    background(img, t)
    title(img, 'A remarkable year', 'id Software and the PC platformer', t)
    x0, x1, y = 180, 1740, 560

    def X(year):
        return x0 + (year - 1990.4) / (1996.8 - 1990.4) * (x1 - x0)

    with Layer(img, ramp(t, 0.2, 0.6)) as L:
        L.d.line([(x0, y), (x1, y)], fill=EDGE, width=6)
        for yr in range(1991, 1997):
            L.d.line([(X(yr), y - 14), (X(yr), y + 14)], fill=EDGE, width=4)
            L.d.text((X(yr), y + 40), str(yr), font=font(30, 'semibold'), fill=DIM, anchor='mm')
    for i, (year, when, name, sub, line) in enumerate(EVENTS):
        start = s.at[line] + (0.6 if line == 4 else 0.0) + (i - 4) * 0.9 * (line == 4)
        if line == 2 and 'id' in name:
            start = s.at[2] + 2.8
        a = ramp(t, start, 0.5)
        if a <= 0:
            continue
        x = X(year)
        up = i % 2 == 0
        col = YELLOW if 'Keen' in name else (CYAN if line < 4 else SOFT)
        dot(img, (x, y), 12, col, a)
        # Cards sit in evenly spaced slots, alternately above and below the line, with a leader to their date.
        bw = 350
        bx = 90 + i * (W - 180 - bw) / (len(EVENTS) - 1)
        cy = y - 250 if up else y + 100
        with Layer(img, a) as L:
            L.d.line([(x, y - 12 if up else y + 12), (bx + bw / 2, cy + 150 if up else cy)], fill=col, width=3)
        panel(img, (bx, cy, bx + bw, cy + 150), alpha=a, edge=col)
        text(img, (bx + 20, cy + 16), when, 24, 'semibold', col, alpha=a)
        text(img, (bx + 20, cy + 48), name, 27, 'bold', INK, alpha=a, spacing=4)
        if sub:
            text(img, (bx + 20, cy + 116), sub, 21, 'regular', SOFT, alpha=a)


ERA = Scene('era', 'funk', [
    'Keen came out of a remarkable year.',
    ('In September 1990, John Carmack got a PC to scroll as smoothly as a Nintendo, in a demo that recreated Super Mario Bros. 3: '
     'Dangerous Dave in Copyright Infringement.',
     'In September nineteen ninety, John Carmack got a P C to scroll as smoothly as a Nintendo, in a demo that recreated Super Mario '
     'Brothers Three: Dangerous Dave in Copyright Infringement.'),
    ('That December, Commander Keen in Invasion of the Vorticons shipped as shareware through Apogee. Its makers left to found id '
     'Software in February 1991.',
     'That December, Commander Keen in Invasion of the Vorticons shipped as shareware through Apogee. Its makers left to found '
     'Id Software in February nineteen ninety-one.'),
    ('In December 1991 came the Goodbye, Galaxy! pair, Keen 4, Secret of the Oracle, and Keen 5, The Armageddon Machine, now '
     'with AdLib music by Bobby Prince.',
     'In December nineteen ninety-one came the Goodbye, Galaxy pair: Keen four, Secret of the Oracle, and Keen five, The '
     'Armageddon Machine, now with Ad Lib music by Bobby Prince.'),
    ('Then came Wolfenstein 3D in 1992, Doom in 1993 and Quake in 1996.',
     'Then came Wolfenstein 3 D in nineteen ninety-two, Doom in ninety-three, and Quake in ninety-six.'),
], draw_era)


# ---------------------------------------------------------------------------------------------------------------- 5. tiles

TILE_PX = 64


def tile_map():
    """A little cave, in the game's own tiles, for the scrolling demonstration."""
    cols, rows = 48, 9
    ground = [6, 6, 6, 5, 5, 6, 7, 7, 6, 6, 4, 4, 5, 6, 6, 6, 7, 7, 7, 6, 5, 5, 6, 6] * 2
    m = []
    for r in range(rows):
        row = []
        for c in range(cols):
            g = ground[c]
            if r > g:
                row.append('cavesFill')
            elif r == g:
                row.append('cavesTop')
            elif r == g - 1 and c % 7 == 3:
                row.append('cheezie')
            elif r == 1 and c % 5 == 2:
                row.append('cavesBlock')
            else:
                row.append(None)
        m.append(row)
    return m


_TILES = None


def tiles_image():
    global _TILES
    if _TILES is None:
        m = tile_map()
        img = Image.new('RGB', (len(m[0]) * TILE_PX, len(m) * TILE_PX), (18, 16, 30))
        back = sprite('cavesBack', 4).convert('RGB').point(lambda v: v * 4 // 10)
        for r, row in enumerate(m):
            for c, n in enumerate(row):
                img.paste(back, (c * TILE_PX, r * TILE_PX))
                if n:
                    sp = sprite(n, 4)
                    img.paste(sp, (c * TILE_PX, r * TILE_PX), sp)
        _TILES = img
    return _TILES


def draw_tiles(img, t, s):
    background(img, t)
    title(img, 'Adaptive tile refresh', "how Keen scrolled on an EGA card", t)
    world = tiles_image()
    ox, oy = 160, 300
    view_w, view_h = 20 * TILE_PX // 2 + 0, 9 * TILE_PX
    cam = max(0.0, t - s.at[1]) * 70
    cam = min(cam, world.width - 1000)
    # The whole map, dimmed, and the part the screen shows.
    left = max(0, int(cam) - 300)
    strip = world.crop((left, 0, left + 1000, world.height))
    dimmed = strip.point(lambda v: v * 35 // 100)
    img.paste(dimmed, (ox, oy))
    vx = ox + int(cam) - left
    view = world.crop((int(cam), 0, int(cam) + view_w, view_h))
    img.paste(view, (vx, oy))
    a = ramp(t, s.at[1], 0.5)
    with Layer(img, a) as L:
        L.d.rectangle((vx - 4, oy - 4, vx + view_w + 4, oy + view_h + 4), outline=CYAN, width=6)
        L.d.text((vx, oy - 20), 'the screen', font=font(30, 'semibold'), fill=CYAN, anchor='lb')
        # The column that has just come into view is the only one redrawn.
        edge = vx + view_w - (cam % TILE_PX) - TILE_PX
        if t > s.at[1] + 0.5:
            flash = 0.5 + 0.5 * math.sin(t * 10)
            L.d.rectangle((edge, oy, vx + view_w, oy + view_h), fill=YELLOW + (int(60 + 60 * flash),), outline=YELLOW, width=3)
    if t > s.at[2]:
        tx = 1240
        text(img, (tx, 300), 'Redrawn this frame', 34, 'semibold', SOFT, alpha=ramp(t, s.at[2], 0.5))
        text(img, (tx, 350), '9 of 90 tiles', 72, 'black', YELLOW, alpha=ramp(t, s.at[2], 0.5))
        text(img, (tx, 450), 'the rest moved by the card,\nby changing where it starts\nreading video memory', 32, 'regular', INK,
             alpha=ramp(t, s.at[2] + 0.4, 0.5))
    if t > s.at[3]:
        text(img, (1240, 650), 'Lieutenant Fizz redraws all\n64,000 pixels every frame', 32, 'semibold', ORANGE,
             alpha=ramp(t, s.at[3], 0.5))


TILES = Scene('tiles', 'funk', [
    "Keen's secret was adaptive tile refresh.",
    'EGA cards could pan the screen in hardware, so instead of redrawing everything, Keen redrew only the tiles that changed.',
    'On a PC with no sprite hardware at all, that was enough to scroll like a console.',
    ("Lieutenant Fizz can't use that trick: its parallax backdrop and its lighting change every pixel, so it redraws the whole "
     'screen, every frame.',
     "Lieutenant Fizz can't use that trick: its parallax backdrop and its lighting change every pixel, so it redraws the whole "
     'screen, every frame.'),
], draw_tiles)


# ---------------------------------------------------------------------------------------------------------------- 6. memory

def draw_memory(img, t, s):
    background(img, t)
    title(img, 'The 486 and the 640K barrier', 'how a big DOS game found its memory', t)
    # The machine.
    a = ramp(t, s.at[0], 0.5)
    panel(img, (110, 260, 700, 560), alpha=a)
    specs = [('CPU', 'Intel 486DX2, 66 MHz'), ('FPU', 'x87, built in'), ('RAM', '4 or 8 MB'), ('Video', 'VGA'),
             ('Sound', 'Sound Blaster')]
    for i, (k, v) in enumerate(specs):
        text(img, (150, 290 + i * 52), k, 30, 'semibold', SOFT, alpha=ramp(t, s.at[0] + 0.2 * i, 0.4))
        text(img, (290, 290 + i * 52), v, 30, 'medium', INK, alpha=ramp(t, s.at[0] + 0.2 * i, 0.4))
    # The memory map (not to scale).
    x0, x1, y0, y1 = 110, 1810, 640, 760
    segs = [(0.00, 0.27, 'conventional: 0 to 640 KB', 'DOS, drivers and the program', ORANGE, s.at[1]),
            (0.27, 0.48, 'upper: 640 KB to 1 MB', 'VGA at A0000, the BIOS', CYAN, s.at[1] + 1.0),
            (0.48, 1.00, 'extended: above 1 MB', 'only reachable in protected mode', GREEN, s.at[2])]
    for f0, f1, name, sub, col, start in segs:
        a = ramp(t, start, 0.6)
        xa, xb = x0 + (x1 - x0) * f0, x0 + (x1 - x0) * f1
        with Layer(img, a) as L:
            L.d.rectangle((xa, y0, xb - 4, y1), fill=mix(BG, col, 0.35), outline=col, width=3)
            L.d.text((xa + 16, y0 + 18), name, font=font(26, 'bold'), fill=INK)
            L.d.text((xa + 16, y0 + 62), sub, font=font(23, 'regular'), fill=INK)
    # FIZZ.EXE moves up into extended memory with the DOS extender.
    if t > s.at[2]:
        k = ease((t - s.at[3]) / 1.5) if t > s.at[3] else 0.0
        fx = x0 + 60 + k * ((x1 - x0) * 0.62)
        chip_y = 820
        chip(img, (fx, chip_y), 'FIZZ.EXE', YELLOW, ramp(t, s.at[2] + 0.4, 0.5), 32)
        text(img, (800, 300), 'DOS extenders', 44, 'bold', INK, alpha=ramp(t, s.at[2], 0.5))
        text(img, (800, 360), 'Doom: DOS/4GW', 34, 'medium', SOFT, alpha=ramp(t, s.at[2] + 0.5, 0.5))
        text(img, (800, 410), 'Quake: CWSDPMI', 34, 'medium', SOFT, alpha=ramp(t, s.at[3], 0.5))
        text(img, (800, 460), 'Lieutenant Fizz: CWSDPMI', 34, 'bold', YELLOW, alpha=ramp(t, s.at[3] + 0.6, 0.5))


MEMORY = Scene('memory', 'crater', [
    ('By 1993, the machine to have was a 486DX2 at 66 megahertz, with four or eight megabytes of memory.',
     'By nineteen ninety-three, the machine to have was a four eighty-six D X two at sixty-six megahertz, with four or eight '
     'megabytes of memory.'),
    'But DOS itself only ran programs in the first 640 kilobytes.',
    ('Big games escaped with DOS extenders, which switch the processor into 32-bit protected mode. Doom used DOS/4GW.',
     'Big games escaped with DOS extenders, which switch the processor into thirty-two bit protected mode. Doom used DOS four G W.'),
    ('Lieutenant Fizz uses CWSDPMI, the same extender Quake shipped with.',
     'Lieutenant Fizz uses C W S D P M I, the same extender Quake shipped with.'),
], draw_memory)


# ---------------------------------------------------------------------------------------------------------------- 7. mode 13h

_BUSY = {}


def busy_window(idx, period, cols, rows):
    """A window of the frame with many different bytes in it, chosen afresh every few seconds (so it holds still between)."""
    if period not in _BUSY:
        best = (-1, 0, 0)
        for y in range(40, 200 - rows, 4):
            for x in range(0, 320 - cols, 4):
                n = len(np.unique(idx[y:y + rows, x:x + cols]))
                if n > best[0]:
                    best = (n, x, y)
        _BUSY[period] = best[1:]
    return _BUSY[period]


def draw_vga(img, t, s):
    background(img, t)
    title(img, 'VGA mode 13h', '320 × 200, 256 colours, a byte a pixel', t)
    frame = clip('day').frame(t, loop=True, span=13)
    box = (110, 260, 1070, 860)
    monitor(img, frame, box)
    # A small window of the frame, magnified, with the bytes behind it.
    cols, rows = 8, 6
    raw = clip('day').files[int(t * 30) % (13 * 30)].read_bytes()
    idx = np.frombuffer(raw, np.uint8, 64000).reshape(200, 320)
    sx, sy = busy_window(idx, int(t / 2.5), cols, rows)
    scale = (box[2] - box[0]) / 320
    a = ramp(t, s.at[1], 0.5)
    with Layer(img, a) as L:
        L.d.rectangle((box[0] + sx * scale, box[1] + sy * scale, box[0] + (sx + cols) * scale, box[1] + (sy + rows) * scale),
                      outline=YELLOW, width=4)
    cell = 82
    gx, gy = 1150, 280
    if a > 0:
        with Layer(img, a) as L:
            for r in range(rows):
                for c in range(cols):
                    rgb = tuple(int(v) for v in frame[sy + r, sx + c])
                    x, y = gx + c * cell, gy + r * cell
                    L.d.rectangle((x, y, x + cell - 4, y + cell - 4), fill=rgb)
                    lum = 0.3 * rgb[0] + 0.59 * rgb[1] + 0.11 * rgb[2]
                    L.d.text((x + cell / 2 - 2, y + cell / 2 - 2), f'{idx[sy + r, sx + c]:02X}', font=font(26, 'monobold'),
                             fill=(0, 0, 0) if lum > 110 else INK, anchor='mm')
    text(img, (1150, 800), 'address = A0000 + y × 320 + x', 34, 'monobold', YELLOW, alpha=ramp(t, s.at[1] + 0.8, 0.5))
    text(img, (1150, 200), 'each byte picks one of 256 colours', 30, 'medium', SOFT, alpha=a)


VGA = Scene('vga', 'sky', [
    ('For graphics there was VGA mode 13h: 320 by 200 pixels, 256 colours, one byte for each pixel.',
     'For graphics there was V G A mode thirteen H: three twenty by two hundred pixels, two hundred and fifty-six colours, one '
     'byte for each pixel.'),
    ('The whole screen is 64,000 bytes, starting at memory address A0000.',
     'The whole screen is sixty-four thousand bytes, starting at memory address hex A zero zero zero zero.'),
    'Write a byte there and a pixel lights up. This port builds each frame in memory and copies it across once a frame.',
], draw_vga)


# ---------------------------------------------------------------------------------------------------------------- 8. sound

CH_NAMES = ['1', '2', '3', '4', '5', '6', '7', '8', '9', 'BD', 'SD', 'TT', 'CY', 'HH']


def draw_sound(img, t, s):
    background(img, t, (40, 20, 52))
    title(img, 'AdLib, Sound Blaster and the beeper', 'the Yamaha OPL2: FM synthesis on a card', t)
    act = assets.channel_activity(assets.song_index(s.song))
    f = int(s.song_t(t) * 30) % len(act)
    a = ramp(t, s.at[0] + 0.5, 0.5)
    x0, base = 150, 640
    with Layer(img, a) as L:
        for i, name in enumerate(CH_NAMES):
            x = x0 + i * 82 + (40 if i >= 9 else 0)
            v = float(act[f, i])
            col = ORANGE if i < 5 else (DIM if i < 9 else MAGENTA)
            L.d.rectangle((x, base - 300, x + 56, base), fill=(26, 30, 46))
            L.d.rectangle((x, base - 300 * v, x + 56, base), fill=col)
            L.d.text((x + 28, base + 30), name, font=font(26, 'semibold'), fill=SOFT, anchor='mm')
        L.d.text((x0, base + 80), 'melody voices 1–5 · voice 6 for effects · rhythm-mode drums', font=font(26, 'regular'), fill=SOFT)
    # The PC speaker: one square wave.
    if t > s.at[2]:
        a2 = ramp(t, s.at[2], 0.5)
        with Layer(img, a2) as L:
            L.d.text((1420, 330), 'PC speaker', font=font(36, 'bold'), fill=CYAN)
            pts = []
            for i in range(0, 380, 2):
                ph = (i + t * 160) / 38
                pts.append((1420 + i, 470 - 60 * (1 if int(ph) % 2 else -1)))
            L.d.line(pts, fill=CYAN, width=5)
    if t > s.at[3]:
        k = (t - s.at[3] - 0.3) / 0.07
        line = 'SET BLASTER=A220 I7 D1 T3'
        typed = line[:max(0, int(k))]
        panel(img, (1180, 620, 1820, 720), fill=(4, 6, 8), edge=EDGE, alpha=ramp(t, s.at[3], 0.4))
        text(img, (1210, 650), 'C:\\>' + typed, 36, 'mono', (200, 205, 210), alpha=ramp(t, s.at[3], 0.4))


SOUND = Scene('sound', 'caves', [
    ('Music came from an AdLib or Sound Blaster card and its Yamaha OPL2 chip: nine voices of FM synthesis, or six and five drums.',
     'Music came from an Ad Lib or Sound Blaster card and its Yamaha O P L two chip: nine voices of F M synthesis, or six and five '
     'drums.'),
    "This is the game's own music, played on that chip, and each bar is one of its voices.",
    'Without a card you had the PC speaker: a single square wave.',
    ('And if you ever set up a Sound Blaster, you remember a line like this one. The browser version of this game uses exactly '
     'these settings.',
     'And if you ever set up a Sound Blaster, you remember a line like this one. The browser version of this game uses exactly '
     'these settings.'),
], draw_sound)


# ---------------------------------------------------------------------------------------------------------------- 9. the port

STEPS = [
    ('Rust', "the game's crates,\nshared with the web", ORANGE),
    ('no_std', 'core, alloc and\nlibm maths', ORANGE),
    ('i486', 'rustc, a custom\ntarget spec', YELLOW),
    ('go32', 'coff2go32.py\nfixes the objects', CYAN),
    ('DJGPP', 'gcc links it with\nthe C layer', CYAN),
    ('FIZZ.EXE', 'and CWSDPMI.EXE,\nunder 1 MB', GREEN),
]


def draw_port(img, t, s):
    background(img, t)
    title(img, 'From Rust to FIZZ.EXE', 'the same simulation the web build runs', t)
    n = len(STEPS)
    bw, gap, y = 274, 30, 420
    x0 = (W - (n * bw + (n - 1) * gap)) // 2
    starts = [s.at[1], s.at[1] + 1.2, s.at[2], s.at[2] + 1.5, s.at[3], s.at[3] + 1.6]
    for i, (name, sub, col) in enumerate(STEPS):
        a = ramp(t, starts[i], 0.5)
        x = x0 + i * (bw + gap)
        panel(img, (x, y, x + bw, y + 230), alpha=a, edge=col)
        text(img, (x + bw / 2, y + 60), name, 42, 'black', col, 'mm', alpha=a)
        text(img, (x + bw / 2, y + 150), sub, 24, 'regular', INK, 'mm', alpha=a, align='center')
        if i:
            arrow(img, (x - gap + 6, y + 115), (x - 6, y + 115), SOFT, 4, alpha=a, head=12)
    # The web build, from the same crates.
    a = ramp(t, s.at[1] + 0.6, 0.5)
    arrow(img, (x0 + bw / 2, y + 236), (x0 + bw / 2, y + 330), SOFT, 4, alpha=a)
    chip(img, (x0 + 10, y + 344), 'wasm for the web', BLUE, a, 28)
    # A packet running through the pipeline.
    if t > s.at[1]:
        k = ((t - s.at[1]) / 2.2) % 1.0
        pts = [(x0 + bw / 2, y + 115), (x0 + (n - 1) * (bw + gap) + bw / 2, y + 115)]
        dot(img, along(pts, k), 9, YELLOW, alpha=ramp(t, s.at[1] + 0.5, 0.5))


PORT = Scene('port', 'tower', [
    'So how do you get a modern game onto DOS?',
    ("Lieutenant Fizz is written in Rust, and its simulation, every jump, every collision and every enemy, is shared with the web "
     'version.',
     "Lieutenant Fizz is written in Rust, and its simulation, every jump, every collision and every enemy, is shared with the web "
     'version.'),
    ("For DOS, that code is built for a bare 486 with a hand-written target, without an operating system's standard library.",
     "For DOS, that code is built for a bare four eighty-six with a hand-written target, without an operating system's standard "
     'library.'),
    ("A small converter rewrites Rust's object files so DJGPP's linker can read them, and out comes FIZZ.EXE.",
     "A small converter rewrites Rust's object files so D J G P P's linker can read them, and out comes fizz dot E X E."),
], draw_port)


# ---------------------------------------------------------------------------------------------------------------- 10. parity

def hash_row(img, x, y, n, done, bad_at, label, col, alpha):
    with Layer(img, alpha) as L:
        L.d.text((x - 30, y + 22), label, font=font(30, 'semibold'), fill=col, anchor='rm')
        for i in range(n):
            cx = x + i * 46
            if i < done:
                good = bad_at is None or i < bad_at
                fill = mix(BG, GREEN if good else RED, 0.75)
            else:
                fill = (30, 36, 54)
            L.d.rounded_rectangle((cx, y, cx + 38, y + 44), radius=6, fill=fill)


def draw_parity(img, t, s):
    background(img, t)
    title(img, 'Making the maths agree', '800 checkpoints, compared with the web build', t)
    n = 30
    x, y = 420, 330
    k1 = clamp((t - s.at[1] - 0.5) / 4.0)
    hash_row(img, x, y, n, int(k1 * n), None, 'web build (wasm, f64)', BLUE, ramp(t, s.at[1], 0.5))
    hash_row(img, x, y + 70, n, int(k1 * n), 2, 'DOS, x87 at 64 bits', RED, ramp(t, s.at[1] + 0.3, 0.5))
    if k1 > 0.12:
        text(img, (x + 2 * 46 + 19, y + 140), 'differs at tick 120: two seconds in', 30, 'semibold', RED, 'ma',
             alpha=ramp(t, s.at[1] + 1.2, 0.5))
    k2 = clamp((t - s.at[2] - 0.5) / 4.0)
    hash_row(img, x, y + 260, n, int(k2 * n), None, 'DOS, x87 at 53 bits', GREEN, ramp(t, s.at[2], 0.5))
    chip(img, (x, y + 350), 'fldcw 0x027F', YELLOW, ramp(t, s.at[2] + 0.4, 0.5), 30)
    if k2 >= 1:
        count = min(800, int((t - s.at[2] - 4.5) * 400))
        text(img, (x + 340, y + 352), f'{max(count, 0)} / 800 identical', 40, 'black', GREEN, alpha=ramp(t, s.at[2] + 4.5, 0.4))


PARITY = Scene('parity', 'secret', [
    'The hard part was getting the maths to agree.',
    ("The web version runs on WebAssembly with 64-bit doubles. The 486's floating-point unit normally works in 80 bits, and after "
     'just two seconds of play, the two drift apart.',
     "The web version runs on WebAssembly with sixty-four bit doubles. The four eighty-six's floating point unit normally works in "
     'eighty bits, and after just two seconds of play, the two drift apart.'),
    ('Set the x87 to 53 bits of precision and they agree: 800 checkpoints across all 16 levels, identical to the web build.',
     'Set the x eighty-seven to fifty-three bits of precision, and they agree: eight hundred checkpoints across all sixteen levels, '
     'identical to the web build.'),
], draw_parity, tail=2.0)


# ---------------------------------------------------------------------------------------------------------------- 11. layers

BANDS = [
    ('Simulation', 'm2/sim, m2/game', ['World::step', 'World::render', 'lights'], ORANGE),
    ('Rust runtime', 'm2/dos-rt', ['shell.rs', 'video.rs', 'save.rs'], CYAN),
    ('C layer', 'm4/c, built with DJGPP', ['kbd.c · IRQ 1', 'main.c', 'vga.c · 13h', 'sound.c · IRQ 0'], MAGENTA),
]


def draw_layers(img, t, s):
    background(img, t)
    title(img, 'Three layers', 'the simulation never knows it is on DOS', t)
    starts = [s.at[1], s.at[2], s.at[3]]
    y0, bh, gap = 250, 140, 34
    for i, (name, sub, boxes, col) in enumerate(BANDS):
        a = ramp(t, starts[i], 0.6)
        y = y0 + i * (bh + gap)
        panel(img, (110, y, 1810, y + bh), fill=(16, 22, 40), edge=col, alpha=a)
        text(img, (140, y + 30), name, 40, 'black', col, alpha=a)
        text(img, (140, y + 86), sub, 24, 'regular', SOFT, alpha=a)
        bx0, bx1 = 560, 1780
        bwid = (bx1 - bx0 - (len(boxes) - 1) * 20) / len(boxes)
        for j, b in enumerate(boxes):
            x = bx0 + j * (bwid + 20)
            panel(img, (x, y + 28, x + bwid, y + bh - 28), fill=PANEL, edge=EDGE, alpha=a, radius=12)
            text(img, (x + bwid / 2, y + bh / 2), b, 30, 'semibold', INK, 'mm', alpha=a)
    # Data moving: a key press up from IRQ 1, a frame down to the screen, the timer at 560 Hz.
    if t > s.at[3] + 1.0:
        a = ramp(t, s.at[3] + 1.0, 0.5)
        yb = [y0 + i * (bh + gap) + bh / 2 for i in range(3)]
        key = [(760, yb[2]), (760, yb[1]), (760, yb[0])]
        frame = [(1170, yb[0]), (1170, yb[1]), (1170, yb[2]), (1170, yb[2] + 110)]
        dot(img, along(key, (t * 0.5) % 1), 10, YELLOW, a)
        dot(img, along(frame, (t * 0.45 + 0.3) % 1), 10, CYAN, a)
        pulse = 0.5 + 0.5 * math.sin(t * 2 * math.pi * 2)
        chip(img, (1590, 788), '560 Hz', mix(GREEN, INK, pulse * 0.4), a, 30)
        text(img, (1570, 812), 'IRQ 0, as in Keen', 28, 'medium', SOFT, 'rm', alpha=a)


LAYERS = Scene('layers', 'foundry', [
    'The program has three layers.',
    'The simulation knows nothing about DOS.',
    'A Rust runtime draws each frame and runs the menus, the heads-up display and the saves.',
    ("And a small C layer owns the machine. It takes over the keyboard interrupt, sets the video mode and speeds up the timer to "
     "560 ticks a second for the music: the same rate Commander Keen's music player used.",
     "And a small C layer owns the machine. It takes over the keyboard interrupt, sets the video mode, and speeds up the timer to "
     "five hundred and sixty ticks a second for the music: the same rate Commander Keen's music player used."),
], draw_layers)


# ---------------------------------------------------------------------------------------------------------------- 12. palette

def draw_palette(img, t, s):
    background(img, t)
    title(img, 'A palette trick', '16 EGA colours × 14 brightness levels', t)
    pal = assets.to8(assets.PAL6)
    # The palette, a level a row.
    cell = 34
    gx, gy = 120, 280
    a0 = ramp(t, s.at[1], 0.5)
    if a0 > 0:
        with Layer(img, a0) as L:
            for lvl in range(1, 15):
                ra = ramp(t, s.at[1] + (14 - lvl) * 0.12, 0.3)
                if ra <= 0:
                    continue
                y = gy + (14 - lvl) * (cell + 4)
                for c in range(16):
                    rgb = tuple(int(v) for v in pal[lvl * 16 + c])
                    L.d.rectangle((gx + c * (cell + 4), y, gx + c * (cell + 4) + cell, y + cell), fill=rgb)
            L.d.text((gx, gy + 14 * (cell + 4) + 16), 'byte = level × 16 + colour', font=font(30, 'monobold'), fill=YELLOW)
    # The caves, with the light grid over them.
    name = 'night' if t > s.at[3] else 'caves'
    box = (840, 260, 1800, 860)
    monitor(img, clip(name).frame(t, loop=True), box, alpha=ramp(t, s.at[0], 0.5))
    if s.at[2] < t < s.at[3] + 0.3:
        a = ramp(t, s.at[2] + 0.8, 0.5) * (1 - ramp(t, s.at[3] - 0.2, 0.4))
        with Layer(img, a * 0.8) as L:
            sx = (box[2] - box[0]) / 320
            for gxx in range(0, 321, 8):
                L.d.line([(box[0] + gxx * sx, box[1]), (box[0] + gxx * sx, box[3])], fill=(255, 255, 255, 70), width=1)
            for gyy in range(0, 201, 8):
                L.d.line([(box[0], box[1] + gyy * sx), (box[2], box[1] + gyy * sx)], fill=(255, 255, 255, 70), width=1)
        text(img, (840, 200), 'one brightness for each 8 × 8 block', 30, 'semibold', YELLOW, alpha=a)


PALETTE = Scene('palette', 'caves', [
    'Every frame is a palette trick.',
    'The palette holds the sixteen EGA colours at fourteen brightness levels.',
    ('Sprites are drawn as a colour plus a marker, and a lighting pass turns each marker into a brightness for its eight by eight '
     'block, with a little dithering so it never bands.',
     'Sprites are drawn as a colour plus a marker, and a lighting pass turns each marker into a brightness for its eight by eight '
     'block, with a little dithering so it never bands.'),
    "That's how a 256-colour screen gets glowing crystals and dark caves.",
], draw_palette)


# ---------------------------------------------------------------------------------------------------------------- 13. speed

BARS = [('Caves', 73.2, 43.8), ('Caves at night', 76.6, 47.1), ('Level 9', 83.9, 48.0), ('Overworld map', 55.1, 43.8),
        ('Day level', 37.6, 28.6)]
X87 = [('mov', 1), ('add', 1), ('fmul', 16), ('fistp', 31), ('fdiv', 73), ('fsqrt', 85)]


def draw_speed(img, t, s):
    background(img, t)
    title(img, 'Fast enough for a 486', 'milliseconds a frame in DOSBox at 486DX2-66 speed', t)
    k = ease((t - s.at[3] - 0.3) / 1.5)
    x0, y0, scale = 420, 300, 9.5
    for i, (name, before, after) in enumerate(BARS):
        a = ramp(t, s.at[1] + i * 0.2, 0.4)
        y = y0 + i * 100
        v = before + (after - before) * k
        with Layer(img, a) as L:
            L.d.text((x0 - 24, y + 30), name, font=font(32, 'semibold'), fill=INK, anchor='rm')
            L.d.rectangle((x0, y, x0 + before * scale, y + 60), fill=(40, 30, 40))
            L.d.rectangle((x0, y, x0 + v * scale, y + 60), fill=mix(RED, GREEN, k))
            L.d.text((x0 + v * scale + 18, y + 30), f'{v:.0f} ms', font=font(32, 'bold'), fill=INK, anchor='lm')
    if k >= 1:
        text(img, (x0, y0 + 520), '14 → 23 frames a second in the caves', 40, 'black', GREEN, alpha=ramp(t, s.at[3] + 1.8, 0.5))
    # What an instruction costs a real 486.
    a = ramp(t, s.at[2] + 1.0, 0.5) * (1 - ramp(t, s.at[3] + 0.2, 0.5) * 0.6)
    px, py = 1360, 300
    with Layer(img, a) as L:
        L.d.text((px, py - 50), 'cycles on a real 486', font=font(30, 'semibold'), fill=SOFT)
        for i, (op, c) in enumerate(X87):
            y = py + i * 66
            L.d.text((px + 110, y + 22), op, font=font(30, 'monobold'), fill=INK, anchor='rm')
            L.d.rectangle((px + 130, y + 4, px + 130 + c * 4, y + 42), fill=ORANGE if c > 10 else GREEN)
            L.d.text((px + 140 + c * 4, y + 22), str(c), font=font(28, 'bold'), fill=INK, anchor='lm')


SPEED = Scene('speed', 'theatre', [
    'Speed is the hard part on a 486.',
    'A cave frame used to take 73 milliseconds on an emulated 486: fourteen frames a second.',
    ('Profiling found one backdrop tile drawn 380 times a frame, a pixel at a time, and lighting that used square roots and '
     "divisions, up to 85 cycles each on a real 486's floating-point unit.",
     'Profiling found one backdrop tile drawn three hundred and eighty times a frame, a pixel at a time, and lighting that used '
     "square roots and divisions, up to eighty-five cycles each on a real four eighty-six's floating point unit."),
    'Baking the tiles and switching to integer maths brought that frame down to 44 milliseconds: 23 frames a second.',
], draw_speed, tail=2.0)


# ---------------------------------------------------------------------------------------------------------------- 14. browser

def wave(img, box, t, stretch, glitch, colour, alpha):
    x0, y0, x1, y1 = box
    mid = (y0 + y1) / 2
    pts = []
    for x in range(int(x0), int(x1), 3):
        u = (x - x0) / 60 * stretch + t * 3
        v = math.sin(u) * 0.6 + math.sin(u * 2.7) * 0.25
        if glitch and int(u * 2) % 9 == 0:
            v = 0
        pts.append((x, mid - v * (y1 - y0) / 2))
    with Layer(img, alpha) as L:
        L.d.line(pts, fill=colour, width=4)


def draw_browser(img, t, s):
    background(img, t)
    title(img, 'In your browser', 'DOSBox compiled to WebAssembly, by js-dos', t)
    bx0, by0, bx1, by1 = 110, 230, 976, 870
    with Layer(img, ramp(t, 0.2, 0.5)) as L:
        L.d.rounded_rectangle((bx0, by0, bx1, by1), radius=18, fill=(230, 233, 240))
        L.d.rounded_rectangle((bx0, by0, bx1, by0 + 64), radius=18, fill=(205, 210, 220))
        L.d.rectangle((bx0, by0 + 40, bx1, by0 + 64), fill=(205, 210, 220))
        for i, c in enumerate([(255, 95, 86), (255, 189, 46), (39, 201, 63)]):
            L.d.ellipse((bx0 + 24 + i * 34, by0 + 22, bx0 + 44 + i * 34, by0 + 42), fill=c)
        L.d.rounded_rectangle((bx0 + 150, by0 + 14, bx1 - 30, by0 + 50), radius=14, fill=(255, 255, 255))
        L.d.text((bx0 + 170, by0 + 32), 'scottmorris.github.io/lieutenant-fizz-dos', font=font(24, 'medium'), fill=(60, 66, 80),
                 anchor='lm')
    monitor(img, clip('caves').frame(t + 4, loop=True), (bx0 + 20, by0 + 84, bx1 - 20, by0 + 84 + 516), bezel=False,
            alpha=ramp(t, 0.2, 0.5))
    a = ramp(t, s.at[1], 0.5)
    text(img, (1220, 300), 'prebuffer = 40', 40, 'monobold', RED, alpha=a)
    text(img, (1220, 352), '12% too many samples: spliced, slow music', 28, 'medium', SOFT, alpha=a)
    wave(img, (1220, 400, 1800, 540), t, 1.12, True, RED, a)
    b = ramp(t, s.at[2], 0.5)
    text(img, (1220, 600), 'prebuffer = 0', 40, 'monobold', GREEN, alpha=b)
    text(img, (1220, 652), 'the right number of samples', 28, 'medium', SOFT, alpha=b)
    wave(img, (1220, 700, 1800, 840), t, 1.0, False, GREEN, b)


BROWSER = Scene('browser', 'tiptoe', [
    ("Because it's a real DOS program, it also runs in your browser, inside DOSBox compiled to WebAssembly by js-dos.",
     "Because it's a real DOS program, it also runs in your browser, inside DOSBox compiled to WebAssembly by J S DOS."),
    'Its crackly music came down to one setting: an audio buffer that made the emulator produce twelve percent too many samples.',
    'Set it to zero, and the music plays at the right speed.',
], draw_browser)


# ---------------------------------------------------------------------------------------------------------------- 15. outro

def floppy(img, x, y, alpha):
    with Layer(img, alpha) as L:
        L.d.rounded_rectangle((x, y, x + 420, y + 440), radius=14, fill=(32, 36, 48), outline=(80, 88, 108), width=3)
        L.d.rectangle((x + 100, y, x + 320, y + 130), fill=(150, 156, 168))
        L.d.rectangle((x + 250, y + 18, x + 300, y + 112), fill=(32, 36, 48))
        L.d.rounded_rectangle((x + 50, y + 200, x + 370, y + 420), radius=8, fill=(240, 236, 224))
        for i in range(4):
            L.d.line([(x + 70, y + 290 + i * 34), (x + 350, y + 290 + i * 34)], fill=(190, 196, 210), width=2)
    paste_alpha(img, pixel_text('LT. FIZZ', (200, 40, 40), 5), (x + 210, y + 245), alpha, 'mm')
    text(img, (x + 210, y + 330), 'Episode 1 · 1.44 MB', 26, 'semibold', (40, 44, 56), 'mm', alpha=alpha)


def draw_outro(img, t, s):
    background(img, t, (40, 26, 20))
    end = s.at[2] + 0.3
    if t < end:
        floppy(img, 200, 260, ramp(t, s.at[0], 0.5))
        monitor(img, clip('theatre').frame(t, loop=True), (840, 260, 1800, 860), alpha=ramp(t, 0.2, 0.5))
        text(img, (110, 90), 'Fits on a floppy', 64, 'black', INK, alpha=ramp(t, 0.1, 0.5))
        if t > s.at[1]:
            chip(img, (200, 760), 'free and open source', GREEN, ramp(t, s.at[1], 0.4))
            chip(img, (200, 830), 'plays in your browser', CYAN, ramp(t, s.at[1] + 0.5, 0.4))
        return
    a = ramp(t, end, 0.6)
    logo = pixel_text('LIEUTENANT FIZZ', (255, 255, 85), 12, shadow=(170, 0, 0))
    paste_alpha(img, logo, (W // 2, 300), a, 'mm')
    text(img, (W // 2, 440), 'for DOS', 56, 'black', ORANGE, 'mm', alpha=a)
    text(img, (W // 2, 560), 'C:\\FIZZ> FIZZ', 54, 'mono', INK, 'mm', alpha=ramp(t, end + 0.6, 0.5))
    text(img, (W // 2, 660), 'scottmorris.github.io/lieutenant-fizz-dos', 40, 'semibold', CYAN, 'mm', alpha=ramp(t, end + 1.0, 0.5))
    credits = ('Narration: Piper text to speech, Lessac voice · Music: the game\'s own AdLib score, recorded in DOSBox Staging\n'
               'Commander Keen is a trademark of id Software. Lieutenant Fizz is an independent tribute, not affiliated with id '
               'Software or Apogee.')
    text(img, (W // 2, 790), credits, 24, 'regular', SOFT, 'ma', alpha=ramp(t, end + 1.5, 0.5), align='center')


OUTRO = Scene('outro', 'title', [
    ('Lieutenant Fizz for DOS fits on a single 1.44 megabyte floppy disk.',
     'Lieutenant Fizz for DOS fits on a single one point four four megabyte floppy disk.'),
    "It's free, it's open source, and you can play it in your browser right now.",
    'Thanks for watching. Type FIZZ to play again.',
], draw_outro, tail=6.0)


SCENES = [INTRO, STORY, KEEN, ERA, TILES, MEMORY, VGA, SOUND, PORT, PARITY, LAYERS, PALETTE, SPEED, BROWSER, OUTRO]
