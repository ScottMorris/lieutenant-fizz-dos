// Generated from the Lieutenant Fizz repository's audio data; do not edit.
// Episode 1 sound effects and music as Undertone voice data and mini-notation.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

import type { AudioPatterns, MusicPart, MusicTrack, SfxVoice } from './types.js';

// Sound effects and music for Episode 1, written as Undertone-style voice data and
// mini-notation patterns (ported from the prototype's audio.js).

// ---------- One-shot SFX (Undertone voice controls as data) ----------
const V = (n: string, o: Partial<SfxVoice> = {}): SfxVoice => ({
  n,
  w: 'triangle',
  a: 0.001,
  d: 0.1,
  r: 0.05,
  g: 0.4,
  ...o,
});
export const SFX: Record<string, SfxVoice[]> = {
  jump: [
    V('e4', { w: 'square', d: 0.08, g: 0.18, lpf: 2200 }),
    V('b4', { w: 'square', d: 0.1, g: 0.16, lpf: 2600, nudge: 0.035 }),
  ],
  boing: [
    V('g3', { d: 0.14, g: 0.45 }),
    V('g4', { d: 0.1, g: 0.3, nudge: 0.04 }),
    V('d5', { w: 'sine', d: 0.08, g: 0.2, nudge: 0.07 }),
  ],
  fzzt: [
    V('white', { d: 0.08, g: 0.22, hpf: 3000 }),
    V('c6', { w: 'sine', d: 0.12, g: 0.25, slide: 0.08 }),
  ],
  crunch: [
    V('white', { d: 0.03, g: 0.3, lpf: 2500 }),
    V('c5', { d: 0.1, g: 0.35, lpf: 4000 }),
    V('e5', { d: 0.14, g: 0.35, lpf: 4500, nudge: 0.05 }),
  ],
  soda: [
    V('white', { a: 0.03, d: 0.3, g: 0.18, hpf: 5000 }),
    V('a5', { w: 'sine', d: 0.2, g: 0.2, nudge: 0.05 }),
  ],
  key: [
    V('c5', { d: 0.1, g: 0.35 }),
    V('e5', { d: 0.1, g: 0.35, nudge: 0.07 }),
    V('g5', { d: 0.2, g: 0.35, nudge: 0.14 }),
  ],
  usb: [
    V('g5', { d: 0.1, g: 0.35 }),
    V('c6', { d: 0.1, g: 0.35, nudge: 0.08 }),
    V('e6', { d: 0.1, g: 0.35, nudge: 0.16 }),
    V('g6', { w: 'sine', d: 0.35, g: 0.3, nudge: 0.24 }),
  ],
  stun: [
    V('a4', { w: 'square', d: 0.12, g: 0.2, lpf: 1500, slide: 0.12 }),
    V('white', { d: 0.05, g: 0.15, hpf: 4000 }),
  ],
  plink: [V('c7', { w: 'sine', d: 0.05, g: 0.25 })],
  bonk: [
    V('c2', { d: 0.1, g: 0.6, slide: 0.07, lpf: 400 }),
    V('white', { d: 0.02, g: 0.3, lpf: 3000 }),
  ],
  hurt: [
    V('a3', { w: 'square', d: 0.45, g: 0.3, lpf: 900, slide: 0.4 }),
    V('e3', { w: 'square', d: 0.4, g: 0.25, lpf: 700, slide: 0.35, nudge: 0.15 }),
  ],
  clunk: [V('c2', { d: 0.15, g: 0.6, lpf: 300 }), V('brown', { d: 0.08, g: 0.4, lpf: 800 })],
  click: [
    V('a5', { w: 'sine', d: 0.06, g: 0.3, lpf: 3000 }),
    V('white', { a: 0, d: 0.008, g: 0.12, lpf: 6000 }),
  ],
  life: [
    V('c5', { d: 0.1, g: 0.35 }),
    V('e5', { d: 0.1, g: 0.35, nudge: 0.09 }),
    V('g5', { d: 0.1, g: 0.35, nudge: 0.18 }),
    V('c6', { w: 'sine', d: 0.3, g: 0.35, nudge: 0.27 }),
  ],
  vworp: [
    V('c5', { w: 'sawtooth', d: 0.4, g: 0.18, lpf: 2000, slide: 0.35 }),
    V('g5', { w: 'sine', d: 0.35, g: 0.2, slide: 0.3, nudge: 0.1 }),
  ],
  zap: [V('e5', { w: 'square', d: 0.08, g: 0.12, lpf: 2500, slide: 0.1 })],
  thoom: [
    V('c1', { w: 'sine', d: 0.4, g: 0.9, slide: 0.2 }),
    V('brown', { d: 0.3, g: 0.5, lpf: 600 }),
  ],
  clang: [
    V('a5', { d: 0.4, g: 0.3 }),
    V('e6', { w: 'sine', d: 0.3, g: 0.2 }),
    V('white', { d: 0.05, g: 0.2, hpf: 4000 }),
  ],
  zzzap: [
    V('c6', { w: 'sawtooth', d: 0.3, g: 0.2, lpf: 3000, slide: 0.25 }),
    V('white', { d: 0.2, g: 0.2, hpf: 2000 }),
    V('c3', { w: 'square', d: 0.25, g: 0.2, lpf: 800, nudge: 0.05 }),
  ],
  win: [
    V('c5', { d: 0.1, g: 0.35 }),
    V('e5', { d: 0.1, g: 0.35, nudge: 0.08 }),
    V('g5', { d: 0.1, g: 0.35, nudge: 0.16 }),
    V('c6', { d: 0.1, g: 0.35, nudge: 0.24 }),
    V('e6', { w: 'sine', d: 0.4, g: 0.35, nudge: 0.32 }),
  ],
  beep: [
    V('c6', { w: 'square', d: 0.05, g: 0.15, lpf: 3000 }),
    V('g5', { w: 'square', d: 0.05, g: 0.15, lpf: 3000, nudge: 0.09 }),
    V('e6', { w: 'square', d: 0.08, g: 0.15, lpf: 3000, nudge: 0.18 }),
  ],
  pfff: [V('pink', { a: 0.01, d: 0.22, g: 0.25, lpf: 1500 })],
  snort: [V('brown', { d: 0.15, g: 0.4 }), V('c2', { w: 'sawtooth', d: 0.12, g: 0.25, lpf: 400 })],
  skree: [V('b6', { w: 'sawtooth', d: 0.15, g: 0.1, lpf: 4000, slide: 0.2 })],
  krunch: [V('brown', { d: 0.3, g: 0.55 }), V('c1', { d: 0.2, g: 0.5, slide: 0.1 })],
  bwomp: [V('g2', { w: 'sine', d: 0.18, g: 0.5, slide: 0.15 })],
  sproing: [
    V('c3', { d: 0.2, g: 0.45 }),
    V('c4', { d: 0.15, g: 0.35, nudge: 0.05 }),
    V('g4', { d: 0.15, g: 0.3, nudge: 0.1 }),
    V('c5', { w: 'sine', d: 0.2, g: 0.25, nudge: 0.15 }),
  ],
  poof: [V('white', { a: 0.02, d: 0.15, g: 0.25, lpf: 2000 })],
  splorp: [
    V('e2', { w: 'sine', d: 0.15, g: 0.4, slide: 0.12 }),
    V('pink', { d: 0.06, g: 0.15, lpf: 1200 }),
  ],
  stinger: [
    V('e2', { w: 'sawtooth', a: 0.02, d: 1.1, g: 0.22, lpf: 500 }),
    V('a#2', { w: 'sawtooth', a: 0.02, d: 1.1, g: 0.18, lpf: 500, nudge: 0.03 }),
    V('e5', { w: 'sine', d: 0.8, g: 0.1, nudge: 0.5 }),
    V('brown', { a: 0.05, d: 0.7, g: 0.2, lpf: 700 }),
  ],
  menu: [
    V('a5', { w: 'sine', d: 0.06, g: 0.3, lpf: 3000 }),
    V('white', { a: 0, d: 0.008, g: 0.1, lpf: 6000 }),
  ],
};
export const CAPTION_SFX: Record<string, string> = {
  boing: 'boing',
  fzzt: 'fzzt',
  crunch: 'crunch',
  fsssht: 'soda',
  'red gumdrop': 'key',
  'blue gumdrop': 'key',
  'green gumdrop': 'key',
  'gold USB drive': 'usb',
  fizzled: 'stun',
  plink: 'plink',
  blorp: 'bwomp',
  bonk: 'bonk',
  'whoa!': 'hurt',
  clunk: 'clunk',
  'click-clack': 'click',
  'ding! extra life': 'life',
  vworp: 'vworp',
  clink: 'click',
  ting: 'plink',
  chime: 'key',
  'zap zap zap': 'zap',
  brrrt: 'zap',
  THOOM: 'thoom',
  'CLANG — dome open!': 'clang',
  ZZZAP: 'zzzap',
  'ta-da!': 'win',
  'beep boop': 'beep',
  pfff: 'pfff',
  snort: 'snort',
  thunk: 'clunk',
  skreee: 'skree',
  KRUNCH: 'krunch',
  bwomp: 'bwomp',
  sproing: 'sproing',
  poof: 'poof',
  splorp: 'splorp',
  'click — no fizz': 'plink',
  'needs a drive': 'plink',
  '♪ low sting': 'stinger',
};

// ---------- Music (mini-notation, shared by both back ends) ----------
const P = (notes: string, o: Partial<MusicPart> = {}): MusicPart => ({
  notes,
  w: 'triangle',
  a: 0.005,
  d: 0.12,
  s: 0.3,
  r: 0.08,
  g: 0.2,
  ...o,
});
const kick = (pat: string, g = 0.7): MusicPart =>
  P(pat, { role: 'kick', w: 'sine', d: 0.12, s: 0, r: 0.06, g, slide: 0.09 });
const hats = (pat: string, g = 0.1): MusicPart =>
  P(pat, { role: 'hats', noise: true, d: 0.03, s: 0, r: 0.01, g, hpf: 6000 });
const snare = (pat: string, g = 0.28): MusicPart =>
  P(pat, { role: 'snare', noise: true, d: 0.09, s: 0, r: 0.05, g, hpf: 1600 });
const pad = (notes: string, o: Partial<MusicPart> = {}): MusicPart =>
  P(notes, {
    role: 'pad',
    w: 'sine',
    a: 0.6,
    d: 0.5,
    s: 0.7,
    r: 1.1,
    g: 0.07,
    lpf: 1400,
    room: 0.5,
    ...o,
  });
const bell = (notes: string, o: Partial<MusicPart> = {}): MusicPart =>
  P(notes, {
    role: 'bell',
    w: 'sine',
    a: 0.002,
    d: 0.35,
    s: 0.05,
    r: 0.4,
    g: 0.07,
    delay: 0.4,
    ...o,
  });
export const MUSIC: Record<string, MusicTrack> = {
  // Title: bright C-major theme, eight bars, call-and-answer between lead and bass.
  title: {
    bpm: 140,
    parts: [
      P(
        '<[c5 ~ g4 c5 e5 g5 e5 c5] [a4 ~ e4 a4 c5 e5 c5 a4] [f4 a4 c5 f5 e5 c5 a4 c5] [g4 b4 d5 g5 f5 d5 b4 d5] [e5 ~ e5 g5 c6 ~ b5 g5] [a5 ~ g5 e5 c5 ~ e5 a5] [f5 e5 d5 c5 d5 ~ b4 d5] [c5 ~ g4 ~ c5 ~ ~ ~]>',
        { role: 'lead', w: 'square', g: 0.075, lpf: 2600, d: 0.1 },
      ),
      pad(
        '<[c4,e4,g4] [a3,c4,e4] [f3,a3,c4] [g3,b3,d4] [c4,e4,g4] [a3,c4,e4] [f3,a3,d4] [c4,e4,g4]>',
        { g: 0.05, a: 0.05, room: 0.3 },
      ),
      P(
        '<[c2 c3 g2 c3] [a1 a2 e2 a2] [f1 f2 c2 f2] [g1 g2 d2 g2] [c2 c3 g2 c3] [a1 a2 e2 a2] [f2 f2 g2 g2] [c2 g1 c2 ~]>',
        { role: 'bass', g: 0.3, s: 0.45 },
      ),
      kick('c1 ~ ~ c1 c1 ~ ~ ~', 0.6),
      snare('~ white ~ white', 0.24),
      hats('[~ white]*4', 0.08),
    ],
  },
  // Opening 1: the backyard at night. Music box over a soft pad, crickets.
  yard: {
    bpm: 76,
    parts: [
      bell('<[a5 e5 c5 e5] [b5 e5 c5 e5] [c6 f5 a5 f5] [b5 e5 g#5 e5]>'),
      pad('<[a3,c4,e4] [g3,b3,e4] [f3,a3,c4] [e3,g#3,b3]>'),
      P('<a1 e1 f1 e1>', { role: 'bass', g: 0.16, s: 0.8, r: 0.6 }),
      P('[white ~ white ~ ~ ~ ~ ~ white white ~ ~ ~ ~ ~ ~]', {
        role: 'perc',
        noise: true,
        d: 0.015,
        s: 0,
        r: 0.01,
        g: 0.03,
        hpf: 9000,
      }),
    ],
  },
  // Opening 2: the secret lab. Curious D-dorian arpeggio, stray bleeps.
  lab: {
    bpm: 110,
    parts: [
      P(
        '<[d4 a4 d5 a4 f5 a4 d5 a4] [c4 g4 c5 g4 e5 g4 c5 g4] [a#3 f4 a#4 f4 d5 f4 a#4 f4] [c4 g4 c5 g4 e5 g4 e5 g5]>',
        { role: 'arp', w: 'square', g: 0.045, lpf: 1700, d: 0.08 },
      ),
      bell('~ <d6 a5 f6 e6> ~ ~ ~ ~ <a6 c6> ~', { g: 0.05, d: 0.06, delay: 0.5 }),
      P('<[d2 ~ d2 a1] [c2 ~ c2 g1] [a#1 ~ a#1 f1] [c2 ~ c2 g1]>', {
        role: 'bass',
        g: 0.28,
        s: 0.5,
      }),
      kick('c1 ~ ~ ~ c1 ~ ~ ~', 0.45),
      hats('~ white ~ white', 0.05),
    ],
  },
  // Opening 3: liftoff. Chords climb a step every bar, snare roll on the turn.
  launch: {
    bpm: 128,
    parts: [
      P('<[c5 e5 g5 c6]*2 [d5 f#5 a5 d6]*2 [e5 g#5 b5 e6]*2 [f5 a5 c6 f6]*2>', {
        role: 'arp',
        w: 'square',
        g: 0.06,
        lpf: 2400,
        d: 0.07,
      }),
      P('<[c3,g3,c4,e4] [d3,a3,d4,f#4] [e3,b3,e4,g#4] [f3,c4,f4,a4]>', {
        role: 'pad',
        w: 'sawtooth',
        a: 0.2,
        s: 0.7,
        r: 0.4,
        g: 0.04,
        lpf: 1000,
      }),
      P('<[c2 c2 c3 c2] [d2 d2 d3 d2] [e2 e2 e3 e2] [f2 f2 f3 g2]>', {
        role: 'bass',
        g: 0.3,
        s: 0.4,
      }),
      kick('c1*4', 0.6),
      snare(
        '<[~ white ~ white] [~ white ~ white] [~ white ~ white] [~ white [white white] [white white white white]]>',
        0.22,
      ),
      hats('white*8', 0.07),
    ],
  },
  // Deep space: F lydian, wide pad, sparkling arpeggio, slow melody with echo.
  cine: {
    bpm: 84,
    parts: [
      pad('<[f3,a3,c4,e4] [g3,b3,d4,e4] [a3,c4,e4,g4] [d3,f3,a3,c4]>', { g: 0.08 }),
      P('<[c5 ~ e5 ~ g5 ~ ~ ~] [b4 ~ d5 ~ e5 ~ ~ ~] [c5 ~ e5 ~ a5 ~ g5 ~] [a4 ~ c5 ~ f5 ~ e5 ~]>', {
        role: 'counter',
        w: 'triangle',
        a: 0.02,
        d: 0.3,
        s: 0.2,
        r: 0.4,
        g: 0.07,
        delay: 0.4,
      }),
      bell('<[f5 c6 a5 e6]*2 [g5 d6 b5 e6]*2 [a5 e6 c6 g6]*2 [d5 a5 f5 c6]*2>', {
        g: 0.03,
        d: 0.1,
        s: 0,
        delay: 0.3,
      }),
      P('<f1 g1 a1 d2>', { role: 'drone', w: 'sawtooth', a: 1, s: 0.6, r: 1.5, g: 0.06, lpf: 350 }),
      kick('c1 ~ ~ ~', 0.3),
    ],
  },
  // Overworld: jaunty oom-pah walk in F.
  map: {
    bpm: 112,
    parts: [
      P(
        '<[a4 c5 ~ a4 g4 ~ f4 ~] [g4 a#4 ~ g4 f4 ~ e4 ~] [f4 a4 c5 f5 e5 c5 a4 c5] [d5 ~ c5 ~ a#4 ~ g4 ~] [a4 c5 ~ a4 g4 ~ f4 ~] [g4 a#4 ~ d5 c5 ~ a4 ~] [a#4 d5 f5 d5 c5 a4 g4 e4] [f4 ~ c4 ~ f4 ~ ~ ~]>',
        { role: 'lead', g: 0.13, d: 0.16, s: 0.3 },
      ),
      P(
        '<[~ [a3,c4,f4] ~ [a3,c4,f4]] [~ [g3,c4,e4] ~ [g3,c4,e4]] [~ [a3,c4,f4] ~ [a3,c4,f4]] [~ [a#3,d4,f4] ~ [a#3,d4,f4]] [~ [a3,c4,f4] ~ [a3,c4,f4]] [~ [g3,c4,e4] ~ [g3,c4,e4]] [~ [a#3,d4,f4] ~ [g3,c4,e4]] [~ [a3,c4,f4] ~ ~]>',
        { role: 'counter', w: 'square', g: 0.03, lpf: 1400, d: 0.08, s: 0 },
      ),
      P(
        '<[f2 ~ c3 ~] [c2 ~ g2 ~] [f2 ~ c3 ~] [a#1 ~ f2 ~] [f2 ~ c3 ~] [c2 ~ g2 ~] [a#1 ~ c2 ~] [f2 c2 f2 ~]>',
        { role: 'bass', g: 0.3, s: 0.5 },
      ),
      hats('~ white ~ white', 0.06),
      kick('c1 ~ c1 ~', 0.4),
    ],
  },
  // Crater Fields: driving A-minor run-and-jump.
  crater: {
    bpm: 148,
    parts: [
      P(
        '<[a4 ~ c5 e5 a5 ~ g5 e5] [f5 ~ e5 c5 a4 ~ c5 e5] [d5 ~ f5 a5 g5 f5 e5 d5] [e5 ~ b4 ~ g#4 ~ b4 ~] [a4 ~ c5 e5 a5 ~ b5 c6] [b5 ~ a5 g5 f5 ~ e5 f5] [d5 e5 f5 g5 a5 g5 f5 d5] [e5 ~ ~ ~ e4 ~ ~ ~]>',
        { role: 'lead', w: 'square', g: 0.07, lpf: 2300, d: 0.09 },
      ),
      P(
        '<[a2 a2 a3 a2 a2 a2 a3 g2] [f2 f2 f3 f2 f2 f2 f3 e2] [d2 d2 d3 d2 d2 d2 d3 c2] [e2 e2 e3 e2 e2 e2 e3 g#2]>',
        { role: 'bass', g: 0.3, s: 0.35, d: 0.08 },
      ),
      P('<[c4,e4] [a3,c4] [f3,a3] [g#3,b3]>', {
        role: 'pad',
        w: 'square',
        a: 0.05,
        s: 0.6,
        g: 0.025,
        lpf: 900,
      }),
      kick('c1 ~ ~ c1 c1 ~ ~ ~'),
      snare('~ white ~ white'),
      hats('white*8'),
    ],
  },
  // Crystal Caves: dripping, echoing D minor.
  caves: {
    bpm: 92,
    parts: [
      pad('<[d3,f3,a3] [a#2,d3,f3] [g2,a#2,d3] [a2,c#3,e3]>', { g: 0.09, room: 0.6 }),
      bell('[~ ~ <d6 a5 f6 c#6> ~ ~ ~ ~ ~ ~ <a5 f5> ~ ~ ~ ~ ~ ~]', {
        g: 0.055,
        d: 0.06,
        delay: 0.55,
      }),
      P('<[d5 ~ ~ e5 f5 ~ e5 ~] [d5 ~ ~ ~ a4 ~ ~ ~] [a#4 ~ ~ c5 d5 ~ f5 ~] [e5 ~ ~ ~ c#5 ~ ~ ~]>', {
        role: 'lead',
        w: 'triangle',
        g: 0.07,
        d: 0.3,
        s: 0.3,
        delay: 0.3,
      }),
      P('<d2 a#1 g1 a1>', { role: 'bass', g: 0.24, s: 0.8, r: 0.3 }),
      kick('c1 ~ ~ ~ ~ ~ c1 ~', 0.4),
      hats('~ ~ white ~', 0.04),
    ],
  },
  // Open sky: bright, airy C major with a bouncing lead and a sparkle on top.
  sky: {
    bpm: 140,
    parts: [
      P(
        '<[e5 g5 c6 g5 e5 g5 a5 g5] [d5 f5 b5 f5 d5 f5 g5 f5] [c5 e5 a5 e5 c5 e5 f5 e5] [d5 g5 b5 g5 a5 g5 f5 d5] [e5 g5 c6 g5 e6 c6 g5 e5] [f5 a5 c6 a5 f6 c6 a5 f5] [d5 f5 b5 d6 b5 g5 f5 d5] [c5 ~ e5 ~ g5 ~ c6 ~]>',
        { role: 'lead', w: 'square', g: 0.06, lpf: 2600, d: 0.09 },
      ),
      bell(
        '<[c6 ~ e6 ~ g6 ~ e6 ~] [b5 ~ d6 ~ g6 ~ d6 ~] [a5 ~ c6 ~ e6 ~ c6 ~] [a5 ~ c6 ~ f6 ~ c6 ~]>',
        {
          g: 0.04,
          delay: 0.35,
        },
      ),
      pad(
        '<[c4,e4,g4] [g3,b3,d4] [a3,c4,e4] [f3,a3,c4] [c4,e4,g4] [f3,a3,c4] [g3,b3,d4] [c4,e4,g4]>',
        {
          g: 0.045,
          room: 0.4,
        },
      ),
      P(
        '<[c2 ~ g2 ~] [g1 ~ d2 ~] [a1 ~ e2 ~] [f1 ~ c2 ~] [c2 ~ g2 ~] [f1 ~ c2 ~] [g1 ~ d2 ~] [c2 g1 c2 ~]>',
        { role: 'bass', g: 0.3, s: 0.5 },
      ),
      kick('c1 ~ c1 ~', 0.5),
      hats('~ white ~ white', 0.06),
    ],
  },
  // Towers: a climbing D-minor ostinato that never quite stops rising.
  tower: {
    bpm: 128,
    parts: [
      P(
        '<[d4 f4 a4 d5 a4 f4 d4 f4] [d4 f4 a4 d5 a4 f4 d4 f4] [e4 g4 b4 e5 b4 g4 e4 g4] [f4 a4 c5 f5 c5 a4 f4 a4] [g4 b4 d5 g5 d5 b4 g4 b4] [a4 c#5 e5 a5 e5 c#5 a4 c#5] [d5 f5 a5 d6 a5 f5 d5 f5] [e5 g5 b5 e6 b5 g5 e5 g5]>',
        { role: 'arp', w: 'square', g: 0.055, lpf: 2200, d: 0.08 },
      ),
      P('<[a5 ~ ~ f5 ~ ~ d5 ~] [a5 ~ ~ f5 ~ ~ d5 ~] [b5 ~ ~ g5 ~ ~ e5 ~] [c6 ~ ~ a5 ~ ~ f5 ~]>', {
        role: 'counter',
        w: 'triangle',
        g: 0.05,
        d: 0.12,
        delay: 0.3,
      }),
      pad(
        '<[d3,f3,a3] [d3,f3,a3] [e3,g3,b3] [f3,a3,c4] [g3,b3,d4] [a3,c#4,e4] [d3,f3,a3] [e3,g3,b3]>',
        {
          g: 0.04,
        },
      ),
      P(
        '<[d2 d2 d3 d2] [d2 d2 d3 d2] [e2 e2 e3 e2] [f2 f2 f3 f2] [g2 g2 g3 g2] [a1 a1 a2 a1] [d2 d2 d3 d2] [e2 e2 e3 e2]>',
        { role: 'bass', g: 0.28, s: 0.45 },
      ),
      kick('c1 ~ c1 ~', 0.5),
      snare('~ white ~ white', 0.2),
      hats('white*8', 0.05),
    ],
  },
  // The theatre: a waltz, six steps to the bar, with a wheezy lead.
  theatre: {
    bpm: 108,
    parts: [
      P(
        '<[e5 ~ g5 e5 c5 ~] [d5 ~ f5 d5 b4 ~] [c5 ~ e5 c5 a4 ~] [d5 ~ g5 f5 d5 ~] [e5 ~ g5 c6 g5 e5] [f5 ~ a5 c6 a5 f5] [g5 ~ b5 d6 b5 g5] [c6 ~ ~ g5 ~ ~]>',
        { role: 'lead', w: 'sawtooth', g: 0.05, lpf: 1800, a: 0.02, d: 0.14, s: 0.4 },
      ),
      P(
        '<[~ ~ [c4,e4,g4] ~ [c4,e4,g4] ~] [~ ~ [g3,b3,d4] ~ [g3,b3,d4] ~] [~ ~ [a3,c4,e4] ~ [a3,c4,e4] ~] [~ ~ [g3,b3,d4] ~ [g3,b3,d4] ~]>',
        { role: 'counter', w: 'square', g: 0.035, lpf: 1500, d: 0.08, s: 0 },
      ),
      P('<[c2 ~ ~ ~ ~ ~] [g1 ~ ~ ~ ~ ~] [a1 ~ ~ ~ ~ ~] [g1 ~ ~ ~ ~ ~]>', {
        role: 'bass',
        g: 0.3,
        s: 0.5,
      }),
      hats('[white ~ white white ~ white]', 0.05),
      kick('c1 ~ ~ ~ ~ ~', 0.4),
    ],
  },
  // The foundry: heavy, mechanical E minor, with metallic clanks off the beat.
  foundry: {
    bpm: 132,
    parts: [
      P(
        '<[e1 e1 ~ e1 e1 ~ g1 ~] [e1 e1 ~ e1 e1 ~ b1 ~] [c2 c2 ~ c2 c2 ~ e2 ~] [d2 d2 ~ d2 d2 ~ b1 ~]>',
        { role: 'bass', w: 'sawtooth', g: 0.2, lpf: 600, s: 0.5 },
      ),
      P('<[~ e5 ~ ~ ~ e5 ~ ~] [~ g5 ~ ~ ~ g5 ~ ~] [~ e5 ~ ~ ~ a5 ~ ~] [~ f#5 ~ ~ ~ b5 ~ ~]>', {
        role: 'counter',
        w: 'square',
        g: 0.045,
        lpf: 3200,
        d: 0.05,
        s: 0,
        delay: 0.15,
      }),
      P('<[e3,g3,b3] [e3,g3,b3] [c3,e3,g3] [d3,f#3,a3]>', {
        role: 'pad',
        w: 'sawtooth',
        g: 0.035,
        lpf: 900,
        a: 0.1,
        s: 0.7,
      }),
      kick('c1 ~ c1 c1 c1 ~ c1 ~', 0.7),
      snare('~ ~ white ~ ~ ~ white white', 0.26),
      hats('[white white]*4', 0.07),
      P('[white ~ ~ ~ ~ ~ white ~]', {
        role: 'perc',
        noise: true,
        d: 0.18,
        s: 0,
        r: 0.1,
        g: 0.06,
        lpf: 5000,
        hpf: 2500,
      }),
    ],
  },
  // The secret: a short, playful music box in C major.
  secret: {
    bpm: 150,
    parts: [
      bell(
        '<[g5 e5 c5 e5 g5 c6 ~ g5] [a5 f5 c5 f5 a5 c6 ~ a5] [g5 e5 c5 e5 g5 e6 d6 c6] [d6 b5 g5 b5 d6 g6 ~ ~]>',
        {
          g: 0.09,
          d: 0.2,
          delay: 0.25,
        },
      ),
      P('<[c3 ~ g3 ~] [f2 ~ c3 ~] [c3 ~ g3 ~] [g2 ~ d3 ~]>', { role: 'bass', g: 0.22, s: 0.4 }),
      P(
        '<[~ [c4,e4,g4] ~ [c4,e4,g4]] [~ [a3,c4,f4] ~ [a3,c4,f4]] [~ [c4,e4,g4] ~ [c4,e4,g4]] [~ [g3,b3,d4] ~ [g3,b3,d4]]>',
        { role: 'counter', w: 'square', g: 0.03, lpf: 1800, d: 0.06, s: 0 },
      ),
      kick('c1 ~ ~ ~', 0.3),
      hats('~ white ~ white', 0.04),
    ],
  },
  // Mildred's Citadel: menacing G-minor march with organ chords.
  citadel: {
    bpm: 124,
    parts: [
      P(
        '<[g4 ~ g4 a#4 d5 ~ c5 a#4] [a4 ~ a4 c5 d5 ~ f5 d5] [d#5 ~ d5 c5 a#4 ~ a4 g4] [f#4 ~ a4 c5 d5 ~ ~ ~] [g5 ~ f5 d#5 d5 ~ c5 a#4] [c5 ~ a#4 a4 g4 ~ a4 a#4] [d#5 d5 c5 a#4 a4 g4 f#4 a4] [g4 ~ d4 ~ g4 ~ ~ ~]>',
        { role: 'lead', w: 'square', g: 0.07, lpf: 2000 },
      ),
      P(
        '<[g3,a#3,d4] [f3,a3,d4] [d#3,g3,a#3] [d3,f#3,a3] [g3,a#3,d4] [d#3,g3,c4] [c3,d#3,g3] [d3,f#3,a3]>',
        { role: 'pad', w: 'sine', a: 0.05, s: 0.6, g: 0.05 },
      ),
      P(
        '<[g2 d3 g2 d3] [f2 c3 f2 c3] [d#2 a#2 d#2 a#2] [d2 a2 d2 f#2] [g2 d3 g2 d3] [d#2 a#2 d#2 a#2] [c2 g2 c2 g2] [d2 a2 d2 ~]>',
        { role: 'bass', g: 0.3, s: 0.4 },
      ),
      kick('c1 ~ c1 ~'),
      snare('~ white ~ [white white]', 0.24),
      hats('white*4', 0.07),
    ],
  },
  // Cocoa Colossus: frantic E minor.
  boss: {
    bpm: 172,
    parts: [
      P(
        '<[e5 e5 g5 e5 a5 g5 f#5 d5] [e5 e5 g5 e5 b5 a5 g5 f#5] [c6 b5 a5 g5 a5 g5 f#5 e5] [d#5 ~ f#5 ~ b5 ~ a5 f#5]>',
        { role: 'lead', w: 'sawtooth', g: 0.055, lpf: 1900, d: 0.08 },
      ),
      P(
        '<[e2 e2 e3 e2 e2 e3 d3 e2] [e2 e2 e3 e2 e2 e3 d3 e2] [c2 c2 c3 c2 c2 c3 b2 c2] [b1 b1 b2 b1 b1 b2 a2 d#2]>',
        { role: 'bass', w: 'square', g: 0.12, lpf: 600, s: 0.3 },
      ),
      kick('c1*4', 0.65),
      hats('white*8', 0.09),
      snare('~ white ~ white', 0.3),
    ],
  },
  // Ending: triumphant reprise of the title in C.
  ending: {
    bpm: 116,
    parts: [
      P(
        '<[c5 ~ g4 c5 e5 ~ d5 c5] [f5 ~ e5 d5 c5 ~ a4 c5] [d5 ~ b4 d5 g5 ~ f5 e5] [e5 ~ d5 ~ c5 ~ ~ ~] [e5 ~ f5 g5 a5 ~ g5 f5] [e5 ~ d5 c5 a4 ~ c5 e5] [f5 e5 d5 c5 b4 c5 d5 b4] [c5 g4 c5 e5 g5 c6 ~ ~]>',
        { role: 'lead', w: 'square', g: 0.08, lpf: 2600 },
      ),
      pad(
        '<[c4,e4,g4] [f3,a3,c4] [g3,b3,d4] [c4,e4,g4] [a3,c4,f4] [a3,c4,e4] [f3,g3,b3,d4] [c4,e4,g4]>',
        { g: 0.06, a: 0.1, room: 0.3 },
      ),
      P(
        '<[c2 g2 c3 g2] [f2 c3 f2 c3] [g2 d3 g2 d3] [c2 g2 c2 g2] [f2 c3 f2 c3] [a1 e2 a2 e2] [g1 d2 g2 d2] [c2 g2 c3 ~]>',
        { role: 'bass', g: 0.3, s: 0.4 },
      ),
      kick('c1 ~ c1 ~'),
      hats('white*8', 0.08),
      snare('~ white ~ white'),
    ],
  },
};

CAPTION_SFX['jump'] = 'jump';
CAPTION_SFX['kick'] = 'jump';
CAPTION_SFX['crumble'] = 'krunch';
CAPTION_SFX['pop!'] = 'plink';
CAPTION_SFX['heave'] = 'bonk';

export const PATTERNS: AudioPatterns = { sfx: SFX, music: MUSIC, captionSfx: CAPTION_SFX };
