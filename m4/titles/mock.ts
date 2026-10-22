// The opening titles as the design draws them, ported line for line: the oracle the DOS runtime is checked against.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// A port of `draw()` in the design's `fizz-dos-titles.jsx`, with its expressions in the same order so the same floats give the same pixels
// (`Math.round` where it rounds, `| 0` where it truncates). It draws into a 320 × 200 byte buffer, `level * 16 + colour`, from the repo's
// own sprite sheet and font (`data.ts`), and `frame(n)` applies the palette fade and the flash as the runtime does (`ui::dim`, a white fill).
// The sequence is 12.8 s: Presents 0 to 3.2, Words 3.2 to 6.4, Chase 6.4 to 12.8, at 60 ticks a second (768 a loop).

import { CI, CODES, loadMark, loadSheet, loadUi, type Sprite } from './data.ts';

export const SW = 320;
export const SH = 200;
export const TICKS = 768;
const TOTAL = 12.8;
const P = 0;
const WORDS = 3.2;
const CHASE = 6.4;

// ---- the design's easings and `animate` (animations-v3.jsx) ----
const Easing = {
  easeOutBack: (t: number) => {
    const c1 = 1.70158;
    const c3 = c1 + 1;
    return 1 + c3 * Math.pow(t - 1, 3) + c1 * Math.pow(t - 1, 2);
  },
  easeInOutCubic: (t: number) => (t < 0.5 ? 4 * t * t * t : (t - 1) * (2 * t - 2) * (2 * t - 2) + 1),
  easeOutCubic: (t: number) => (--t) * t * t + 1,
};
type Ease = (t: number) => number;
const animate =
  ({ from, to, start, end, ease }: { from: number; to: number; start: number; end: number; ease: Ease }) =>
  (t: number) => {
    if (t <= start) return from;
    if (t >= end) return to;
    const local = (t - start) / (end - start);
    return from + (to - from) * ease(local);
  };
const MOTION = {
  enter: (s: number, e: number, a: number, b: number) => animate({ from: a, to: b, start: s, end: e, ease: Easing.easeOutBack }),
  slide: (s: number, e: number, a: number, b: number) => animate({ from: a, to: b, start: s, end: e, ease: Easing.easeInOutCubic }),
};
const rnd = (i: number) => {
  const x = Math.sin(i * 127.1 + 311.7) * 43758.5453;
  return x - Math.floor(x);
};
const ri = Math.round;

// ---- the pictures ----
type Pic = { w: number; h: number; p: string };
const sheet = loadSheet();
const ui = loadUi();
const mark = loadMark();
const picOf = (s: Sprite): Pic => ({ w: s.w, h: s.h, p: [...s.px].map((c) => (c === 255 ? '.' : CODES[c]!)).join('') });
const spr: Record<string, Pic> = {};
// The design's `gloop0_1` is the sheet's `gloop1`.
for (const n of ['ben_shoot', 'bat0', 'bat1', 'gloop0', 'gloop1', 'phantom0', 'phantom1', 'drone0', 'drone1', 'hopper0', 'hopper1', 'sentry0', 'sentry1', 'beetle0', 'beetle1'])
  spr[n] = picOf(sheet.get(n)!);
const sheetName = (name: string) => (name === 'ben_shoot' ? name : name.replace(/0_(\d)$/, '$1'));

// The Pen effects from enemies.ts: bubble() and stars(f).
const BUBBLE: Pic = (() => {
  const g = Array(64).fill('.');
  const ell = (cx: number, cy: number, r: number, c: string) => {
    for (let j = 0; j < 8; j++)
      for (let i = 0; i < 8; i++) {
        const dx = (i + 0.5 - cx) / r;
        const dy = (j + 0.5 - cy) / r;
        if (dx * dx + dy * dy <= 1) g[j * 8 + i] = c;
      }
  };
  ell(4, 4, 3.2, 'c');
  ell(4, 4, 2, 'b');
  g[2 * 8 + 3] = 'W';
  g[3 * 8 + 2] = 'W';
  return { w: 8, h: 8, p: g.join('') };
})();
spr.bubble = BUBBLE;

// ---- the frame buffer ----
const fb = new Uint8Array(SW * SH);
const put = (x: number, y: number, c: number, L = 14) => {
  x |= 0;
  y |= 0;
  if (x >= 0 && y >= 0 && x < SW && y < SH) fb[y * SW + x] = L * 16 + c;
};
const fill = (x: number, y: number, w: number, h: number, c: number, L = 14) => {
  for (let j = Math.max(0, y | 0); j < Math.min(SH, (y + h) | 0); j++)
    for (let i = Math.max(0, x | 0); i < Math.min(SW, (x + w) | 0); i++) fb[j * SW + i] = L * 16 + c;
};

function sprite(name: string, x: number, y: number, k = 1, flip = false, white = false) {
  const s = spr[sheetName(name)];
  if (!s) return;
  x = ri(x);
  y = ri(y);
  for (let j = 0; j < s.h; j++)
    for (let i = 0; i < s.w; i++) {
      const ch = s.p[j * s.w + i]!;
      if (ch === '.') continue;
      const c = white && ch !== 'k' ? CI.W! : CI[ch]!;
      const px = flip ? x + (s.w - 1 - i) * k : x + i * k;
      fill(px, y + j * k, k, k, c);
    }
}
function stars(f: number, x: number, y: number) {
  [[2, 3], [8, 1], [13, 4]].forEach(([sx, sy], i) => {
    const xx = (sx! + f * 3 + i) % 15;
    for (const [dx, dy] of [[0, 0], [-1, 0], [1, 0], [0, -1], [0, 1]]) put(x + xx + dx!, y + sy! + dy!, CI.y!);
    put(x + xx, y + sy!, CI.W!);
  });
}
function logo(x: number, y: number) {
  const s = mark;
  x = ri(x);
  y = ri(y);
  for (let j = 0; j < s.h; j++)
    for (let i = 0; i < s.w; i++) {
      const t = s.p.substr((j * s.w + i) * 2, 2);
      if (t[0] !== '.') put(x + i, y + j, CI[t[0]!]!, parseInt(t[1]!, 16));
    }
}

// ---- the Fizz font, as ui.rs draws it ----
const glyph = (ch: string) => ui.font.get(ch.codePointAt(0)!);
const advance = (ch: string) => (ch === ' ' ? 3 : glyph(ch) ? glyph(ch)!.w + 1 : 3);
export const textWidth = (s: string) => [...s].reduce((a, ch) => a + advance(ch), 0);
function glyphs(x: number, y: number, s: string, c: number, k = 1, L = 14) {
  let at = ri(x);
  y = ri(y);
  for (const ch of s) {
    const g = glyph(ch);
    if (g) {
      const { w, rows } = g;
      rows.forEach((bits, row) => {
        for (let col = 0; col < w; col++) if ((bits >> (w - 1 - col)) & 1) fill(at + col * k, y + row * k, k, k, c, L);
      });
    }
    at += advance(ch) * k;
  }
}
const text = (x: number, y: number, s: string, c: number) => {
  glyphs(x + 1, y + 1, s, CI.k!);
  glyphs(x, y, s, c);
};
const textBig = (x: number, y: number, s: string, c: number, k: number) => {
  glyphs(x + k, y + k, s, CI.k!, k);
  glyphs(x, y, s, c, k);
};
const textLogo = (x: number, y: number, s: string, c: number, edge: number, k: number) => {
  glyphs(x, y + 2 * k, s, CI.k!, k);
  glyphs(x, y + k, s, edge, k);
  glyphs(x, y, s, c, k);
};
const centred = (cx: number, y: number, s: string, c: number) => text(cx - (textWidth(s) >> 1), y, s, c);

// ---- the scene ----
const GROUND = 176;
type Enemy = { s: string; x: number; fly: number; ph: number; bounce?: number; hop?: number; slide?: string; w?: number; h?: number; knock?: number; rears?: boolean };
const ENEMIES: Enemy[] = [
  { s: 'gloop0', x: 58, fly: 0, ph: 0.0, bounce: 10 },
  { s: 'hopper0', x: 98, fly: 0, ph: 1.1, hop: 1, slide: 'gloop0' },
  { s: 'beetle0', x: 136, fly: 0, ph: 2.3 },
  { s: 'sentry0', x: 14, fly: 0, ph: 0.7, w: 24 },
  { s: 'bat0', x: 56, fly: 30, ph: 0.4 },
  { s: 'drone0', x: 100, fly: 74, ph: 1.9 },
  { s: 'phantom0', x: 28, fly: 72, ph: 2.8, h: 24 },
];
const EK = 2;
const BK = 3;

type Hit = { s: string; at: number };

/** Draws the scene at time `T` seconds into `fb`; returns the fade's steps and whether the flash is on. */
export function draw(T: number): { flash: boolean; steps: number } {
  const total = TOTAL;
  fb.fill(16); // black at level 1
  const Pr = P;
  const Wd = WORDS;
  const C = CHASE;

  // stars
  for (let i = 0; i < 46; i++) {
    const x = ((((rnd(i) * 330 - T * (3 + (i % 3) * 3)) % 330) + 330) % 330) - 5;
    const L = 6 + ri(8 * Math.abs(Math.sin(T * (1 + rnd(i + 3) * 2) + i)));
    put(x, rnd(i + 1) * 170, CI[['W', 'c', 'y', 'm', 'b'][i % 5]!]!, L);
  }

  // Lieutenant / Fizz rows drifting left and right, dim (low brightness levels).
  const boost = MOTION.slide(Wd, Wd + 1.4, 0, 150)(T) + MOTION.slide(Wd + 1.4, C, 0, -100)(T);
  const rowL = ri(MOTION.slide(0, 0.6, 1, 5)(T) - MOTION.slide(C, C + 0.8, 0, 2)(T));
  for (let i = 0; i < 5; i++) {
    const dir = i % 2 ? 1 : -1;
    const w = i % 2 ? 'Fizz' : 'Lieutenant';
    const travel = (T * (26 + i * 6) + boost * (1 + i * 0.2)) * dir;
    const base = dir < 0 ? -20 - i * 50 : -900 + i * 40;
    glyphs(base + travel, 2 + i * 40, Array(8).fill(w).join('  '), dir < 0 ? CI.b! : CI.m!, 5, Math.max(1, rowL));
  }

  // ground + chase
  const gy = ri(GROUND + MOTION.slide(C + 0.1, C + 0.7, 30, 0)(T));
  if (gy < SH) {
    fill(0, gy, SW, SH - gy, CI.M!, 9);
    for (let i = 0; i < 90; i++) {
      const x = ((((rnd(i + 70) * 340 - T * 150) % 340) + 340) % 340) - 10;
      fill(x, gy + 3 + rnd(i + 90) * 22, 2, 1, i % 3 ? CI.B! : CI.b!, 10);
    }
    fill(0, gy, SW, 2, CI.g!);
    fill(0, gy + 2, SW, 1, CI.G!);
  }
  // speed lines
  const sl = MOTION.slide(C + 0.4, C + 0.8, 0, 1)(T);
  if (sl > 0)
    for (let i = 0; i < 10; i++) {
      const len = 20 + rnd(i + 40) * 44;
      const x = SW - ((T * (260 + rnd(i) * 160) + rnd(i + 5) * SW) % (SW + len));
      fill(x, 82 + rnd(i + 20) * 90, len, 1, CI.c!, ri(3 + sl * 6));
    }

  // Ben: ben_shoot at 3x, facing left (back at the enemies).
  const bw = 16 * BK;
  const bh = 24 * BK;
  const benX = ri(MOTION.enter(C + 0.3, C + 1.2, -120, 252)(T) + Math.sin(T * 1.3) * 7);
  const run = Math.abs(Math.sin(T * Math.PI * 3.2));
  const benY = gy - bh - ri(run * 8);
  // Muzzle: sprite pixel (16, 11) mirrored to x 0 — the barrel tip on the left edge.
  const muzzle = (t: number): [number, number] => {
    const bx = ri(MOTION.enter(C + 0.3, C + 1.2, -120, 252)(t) + Math.sin(t * 1.3) * 7);
    const r = Math.abs(Math.sin(t * Math.PI * 3.2));
    return [bx - 1, GROUND - bh - ri(r * 8) + 11 * BK + 1];
  };

  // enemies. A stun (from `hits`) grounds an enemy and knocks it back a little; the yellow hopper is knocked
  // back furthest, so it drops out of the line of fire and the next bubble reaches the green gloop behind it.
  const STUN = 0.9;
  const GROUND_SPEED = 150;
  const hopperState = (t: number, hits: Hit[]) => {
    let h: Hit | undefined;
    for (const c of hits) if (c.s === 'hopper0' && c.at <= t) h = c;
    if (!h) return null;
    const g = hits.find((c) => c.s === 'gloop0' && c.at > h!.at);
    return { h, g, rec: Math.min(h.at + 1.6, g ? g.at + 0.75 : Infinity) };
  };
  const hopperDown = (t: number, hits: Hit[]) => {
    const hs = hopperState(t, hits);
    return hs && t < hs.rec ? hs : null;
  };
  const stunOf = (s: string, t: number, hits: Hit[]) => {
    let best: Hit | undefined;
    for (const h of hits) if (h.s === s && t >= h.at && t < h.at + STUN) best = h;
    return best;
  };
  const enemyAt = (e: Enemy, i: number, t: number, hits: Hit[]) => {
    const enter = C + 0.6 + i * 0.12;
    let x = MOTION.slide(enter, enter + 1.0, e.x - 260, e.x)(t) + Math.sin(t * 2.1 + e.ph) * 9;
    let hop = e.hop ? Math.abs(Math.sin(t * 4 + e.ph)) * 22 : Math.abs(Math.sin(t * (e.bounce ? 5 : 7) + e.ph)) * (e.bounce || 2);
    let st = stunOf(e.s, t, hits);
    const fly = e.fly;
    // With the hopper down in front of it, the gloop rears up into the line of fire.
    if (e.rears) {
      const hs = hopperDown(t, hits);
      if (hs && t > hs.h.at + 0.15) hop = Math.abs(Math.sin((t - hs.h.at) * 7)) * 24;
    }
    if (e.slide) {
      // Stunned on the moving ground, it slides back with it (the ground's 150 px/s) until the gloop behind it is
      // stunned too, then springs forward to its place in the pack.
      st = undefined;
      const hs = hopperState(t, hits);
      if (hs) {
        const { h, rec } = hs;
        const back = (tt: number) => -Math.min(GROUND_SPEED * Math.max(0, tt - h.at - 0.08), 84);
        if (t < rec) {
          st = h;
          const k = Math.min(1, (t - h.at) / 0.2);
          const hopAtHit = Math.abs(Math.sin(h.at * 4 + e.ph)) * 22;
          hop = Math.max(0, hopAtHit * (1 - k) + Math.sin(k * Math.PI) * 6);
          x += back(t);
        } else if (t < rec + 0.5) {
          const k = (t - rec) / 0.5;
          x += back(rec) * (1 - Easing.easeOutCubic(k));
          hop = Math.sin(k * Math.PI) * 34;
        }
      }
    } else if (st) {
      const k = Math.min(1, (t - st.at) / 0.25);
      x -= (e.knock || 6) * Math.sin((k * Math.PI) / 2);
      // a little hop up off the hit, then down onto the ground for the rest of the stun
      const hopAtHit = e.hop ? Math.abs(Math.sin(st.at * 4 + e.ph)) * 22 : 0;
      hop = e.hop ? Math.max(0, hopAtHit * (1 - k) + Math.sin(k * Math.PI) * 6) : 0;
    }
    const h = (e.h || 16) * EK;
    const y = ri(GROUND - h - fly - hop + (fly ? Math.sin(t * 3 + e.ph) * 6 : 0));
    return { e, x: ri(x), y, w: (e.w || 16) * EK, h, st };
  };

  // Bubbles fly dead straight at the muzzle's height, 16 tiles (256 px) a second, as the sim fires them,
  // and stun the first enemy whose box they run into. They are run in the order they were fired, so each
  // one sees the stuns (and knock-backs) the earlier ones caused.
  const SPEED = 256;
  const STEP = 1 / 60;
  const HOP = ENEMIES.findIndex((e) => e.slide);
  const hits: Hit[] = [];
  const live: [number, number, number][] = [];
  // Ben times his fizz to the hopper: while it is in the pack he fires so the bubble meets it at the top of a jump
  // (|sin(4t + ph)| = 1); otherwise he keeps his steady cadence of a shot every 0.32 s.
  const nextShot = (t: number) => {
    const hs = hopperState(t, hits);
    const away = hs && t < hs.rec + 0.5;
    if (away || t < C + 1.2) return t;
    const he = ENEMIES[HOP]!;
    for (let k = Math.ceil(((t + 0.3) * 4 + he.ph - Math.PI / 2) / Math.PI); k < 999; k++) {
      const peak = (Math.PI / 2 + k * Math.PI - he.ph) / 4;
      let t0 = peak - 0.45;
      for (let n = 0; n < 3; n++) {
        const [mx] = muzzle(t0);
        const p = enemyAt(he, HOP, peak, hits);
        t0 = peak - (mx - 8 - (p.x + p.w)) / SPEED;
      }
      if (t0 >= t) return t0;
    }
    return t;
  };
  let tNext = C + 1.3;
  for (let i = 0; i < 24; i++) {
    const t0 = nextShot(tNext);
    const age = T - t0;
    tNext = t0 + 0.32;
    if (age < 0 || t0 > C + 6.2) break;
    // Ben holds his fire from the gloop's stun until the hopper has landed back in the pack, so nothing in flight
    // meets the hopper mid-jump; the shots fired before it have long cleared the screen by the time it jumps.
    const hs = hopperState(t0, hits);
    if (hs && hs.g && t0 >= hs.g.at && t0 < hs.rec + 0.6) continue;
    const [mx, my] = muzzle(t0);
    let hit: Hit | null = null;
    for (let a = 0; a <= Math.min(age, 1.4) && !hit; a += STEP) {
      const bx = mx - 4 - SPEED * a;
      ENEMIES.forEach((e, j) => {
        if (hit) return;
        const p = enemyAt(e, j, t0 + a, hits);
        if (bx - 4 <= p.x + p.w && bx + 4 >= p.x && my + 3 >= p.y && my - 3 <= p.y + p.h) hit = { s: e.s, at: t0 + a };
      });
    }
    if (hit) {
      hits.push(hit);
      continue;
    }
    const bx = mx - 4 - SPEED * age;
    if (bx >= -10) live.push([bx, my, age]);
  }
  live.forEach(([bx, my, age]) => sprite('bubble', bx - 4, my - 4, age < 0.05 ? 1 : 2));

  const order = ENEMIES.map((_, j) => j).sort((a, b) => (ENEMIES[a]!.slide ? 1 : 0) - (ENEMIES[b]!.slide ? 1 : 0));
  order.forEach((j) => {
    const e = ENEMIES[j]!;
    const p = enemyAt(e, j, T, hits);
    p.y += gy - GROUND;
    const since = p.st ? T - p.st.at : undefined;
    const f = p.st ? 0 : Math.floor((T + e.ph) * 6) % 2;
    sprite(`${e.s}_${f}`, p.x, p.y, EK, false, since !== undefined && since < 0.08);
    if (p.st) stars(Math.floor(T * 8) % 3, p.x + (p.w >> 1) - 8, p.y - 8);
  });

  if (T > C + 0.25) {
    fill(benX + 6, gy - 1, bw - 12 - ri(run * 8), 2, CI.k!, 1);
    sprite('ben_shoot', benX, benY, BK, true);
  }

  // Liminal HQ presents
  const out = Wd - 0.5;
  if (T < Wd) {
    const lx = MOTION.enter(Pr + 0.2, Pr + 1.0, -120, 0)(T) + MOTION.slide(out, out + 0.5, 0, -260)(T);
    const name = 'Liminal HQ';
    const k = 3;
    const nw = textWidth(name) * k;
    const tot = 48 + 10 + nw;
    const x0 = (SW - tot) >> 1;
    logo(x0 + lx, 62);
    const nx = MOTION.enter(Pr + 0.7, Pr + 1.4, 280, 0)(T) + MOTION.slide(out + 0.05, out + 0.5, 0, 300)(T);
    if (T > Pr + 0.7) textBig(x0 + 58 + nx, 74, name, CI.W!, k);
    const py = MOTION.enter(Pr + 1.5, Pr + 2.0, 10, 0)(T);
    if (T > Pr + 1.5 && T < out + 0.3) centred(SW / 2, 124 + py, 'presents', CI.y!);
  }

  // The title, as shell.rs title_screen lays it out: kicker at 12, logo at 24 (3x), line at 56.
  if (T > Wd) {
    const words = ui.words;
    const k = 3;
    const wx = (SW - textWidth(words) * k) >> 1;
    const lW = (textWidth('Lieutenant') + 3) * k;
    const drop = ri(MOTION.slide(C, C + 0.8, 60, 0)(T));
    const lx = ri(MOTION.enter(Wd + 0.1, Wd + 0.7, 360, 0)(T));
    const fx = ri(MOTION.enter(Wd + 0.6, Wd + 1.2, -360, 0)(T));
    textLogo(wx + lx, 24 + drop, 'Lieutenant', CI.W!, CI.r!, k);
    textLogo(wx + lW + fx, 24 + drop, 'Fizz', CI.W!, CI.r!, k);
    if (T > Wd + 1.7) centred(SW / 2 + ri(MOTION.enter(Wd + 1.7, Wd + 2.2, -60, 0)(T)), 12 + drop, ui.kicker, CI.y!);
    if (T > C + 1.4) centred(SW / 2, 56 + ri(MOTION.enter(C + 1.4, C + 1.9, 6, 0)(T)), ui.line, CI.c!);
  }

  // Flash: the palette goes to white for a few frames.
  const fl = Wd + 1.2;
  const flash = T > fl && T < fl + 0.07;
  // Palette fade in and out, a brightness level at a time.
  const fade = Math.min(MOTION.slide(0, 0.4, 0, 1)(T), 1 - MOTION.slide(total - 0.6, total, 0, 1)(T));
  return { flash, steps: ri((1 - fade) * 13) };
}

/** Tick `n` of the loop as the screen shows it: the scene, then the fade (`ui::dim`'s rule), then the flash. */
export function frame(n: number): Uint8Array {
  const { flash, steps } = draw(n / 60);
  const out = new Uint8Array(SW * SH);
  for (let i = 0; i < out.length; i++) {
    const v = fb[i]!;
    out[i] = flash ? 14 * 16 + 15 : Math.max(1, (v >> 4) - steps) * 16 + (v & 15);
  }
  return out;
}
