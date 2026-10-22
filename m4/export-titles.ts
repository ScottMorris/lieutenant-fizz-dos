// Writes the opening titles' data: the Liminal HQ mark and the parts of the chase the runtime would rather read than work out.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//   bun m4/export-titles.ts > m4/assets/titles.bin    (reads m4/titles/, the sprite sheet and ui_data.rs; the main game is not needed)
//   bun m4/export-titles.ts --print                   (prints the shot table and the sizes, writes nothing)
//
// The oracle (`titles/mock.ts`) draws the sequence from a clock in float seconds; Ben's running, the enemies' hops and bobs and stuns, the
// bubbles and their hits, and the stars' twinkle all come from sines and from a small simulation of the shots, which the runtime cannot reproduce
// byte for byte without floats. Each of those is sampled here at every tick of the loop (T = n / 60) and stored as the integers the screen needs.
// What is plain arithmetic on the clock (the tweens, the drifts of the stars, specks and speed lines, the fade) is not stored: the runtime works
// those out, and `check-titles.ts` compares its frames with the oracle's.
//
// Layout (little endian), 768 ticks a loop, `chase` the first tick with anything of the chase in view:
//   "FZTI", u8 version 1, u8 enemies, u16 ticks, u16 chase
//   u32 offsets from the start of the file: mark, twinkle, global, enemies, bubble index, bubbles, shots
//   mark         u8 w, u8 h, then w * h bytes of `level * 16 + colour` (0 where clear)
//   twinkle      ticks × 23 bytes: the 46 stars' levels less 6 (0..8), two to a byte, star 2k in the low nibble
//   global       ticks × 20 bytes: i16 x of each of the five rows of words, u8 their level, u8 Ben drawn, i16 ground top,
//                i16 Ben's x, i16 Ben's y, u8 his shadow's width, u8 0
//   enemies      (ticks - chase) × enemies × 6 bytes, in the order they are drawn: i16 x, i16 y, u8 flags (1 second frame, 2 stunned,
//                4 white from a hit), u8 the frame of the stars over a stunned one
//   bubble index (ticks - chase + 1) × u16: where each tick's bubbles begin in the bubbles, the last being the count
//   bubbles      count × 5 bytes: i16 x, i16 y (the sprite's top left), u8 scale (1 or 2)
//   shots        u8 count, then × 9 bytes: u16 ms it leaves, u16 ms it hits (65535 for a miss), u8 what it hits (an enemy's number, 255 for
//                nothing), i16 x and i16 y of the muzzle; for review and for checks, not for drawing

import { writeSync } from 'node:fs';
import { newProbe, draw, TICKS, type Probe } from './titles/mock.ts';
import { loadMark, CI } from './titles/data.ts';

const ENEMY_NAMES = ['gloop0', 'hopper0', 'beetle0', 'sentry0', 'bat0', 'drone0', 'phantom0'];
const STARS = 46;

const probes: Probe[] = [];
for (let n = 0; n < TICKS; n++) {
  const p = newProbe();
  draw(n / 60, p);
  probes.push(p);
}
// The shot table is the one at the end of the loop, where every shot's fate is known. The mock works the shots out afresh at each tick from
// the hits found so far, so a tick's own probe is what that tick draws; the table is for review.
const last = newProbe();
draw(12.8, last);

// What counts as in view: an enemy's box, Ben, or a bubble on the screen.
const SW = 320;
const SH = 200;
const inView = (p: Probe) =>
  p.ben.on || p.bubbles.length > 0 || p.enemies.some((e) => e.x + e.w > 0 && e.x < SW && e.y + e.h > 0 && e.y < SH);
const chase = probes.findIndex(inView);
if (chase < 0) throw new Error('Nothing of the chase is ever in view');
if (probes.slice(0, chase).some(inView)) throw new Error('Something is in view before the chase starts');

const out: number[] = [];
const u8 = (v: number) => {
  if (v < 0 || v > 255 || !Number.isInteger(v)) throw new Error(`${v} is not a byte`);
  out.push(v);
};
const i16 = (v: number) => {
  if (v < -32768 || v > 32767 || !Number.isInteger(v)) throw new Error(`${v} is not an i16`);
  out.push(v & 255, (v >> 8) & 255);
};
const u16 = (v: number) => {
  if (v < 0 || v > 65535 || !Number.isInteger(v)) throw new Error(`${v} is not a u16`);
  out.push(v & 255, v >> 8);
};

const sections: Record<string, number> = {};
const sizes: Record<string, number> = {};
const begin = (name: string) => (sections[name] = out.length);
const end = (name: string) => (sizes[name] = out.length - sections[name]!);

// The header, patched once the offsets are known.
[...'FZTI'].forEach((c) => u8(c.charCodeAt(0)));
u8(1);
u8(ENEMY_NAMES.length);
u16(TICKS);
u16(chase);
const names = ['mark', 'twinkle', 'global', 'enemies', 'bubbles index', 'bubbles', 'shots'];
const tableAt = out.length;
for (let i = 0; i < names.length * 4; i++) u8(0);

begin('mark');
const mark = loadMark();
u8(mark.w);
u8(mark.h);
for (let i = 0; i < mark.w * mark.h; i++) {
  const t = mark.p.substr(i * 2, 2);
  u8(t[0] === '.' ? 0 : parseInt(t[1]!, 16) * 16 + CI[t[0]!]!);
}
end('mark');

begin('twinkle');
for (const p of probes) {
  for (let i = 0; i < STARS; i += 2) {
    const a = p.twinkle[i]! - 6;
    const b = p.twinkle[i + 1]! - 6;
    if (a < 0 || a > 8 || b < 0 || b > 8) throw new Error(`A star's level is outside 6..14: ${p.twinkle[i]}, ${p.twinkle[i + 1]}`);
    u8(a | (b << 4));
  }
}
end('twinkle');

begin('global');
for (const p of probes) {
  p.rowX.forEach(i16);
  u8(p.rowL);
  u8(p.ben.on ? 1 : 0);
  i16(p.gy);
  i16(p.ben.x);
  i16(p.ben.y);
  u8(p.ben.shadow);
  u8(0);
}
end('global');

begin('enemies');
for (const p of probes.slice(chase)) {
  if (p.enemies.length !== ENEMY_NAMES.length) throw new Error('An enemy is missing from a tick');
  for (const e of p.enemies) {
    i16(e.x);
    i16(e.y);
    u8((e.frame ? 1 : 0) | (e.stunned ? 2 : 0) | (e.white ? 4 : 0));
    u8(e.stars);
  }
}
end('enemies');

begin('bubbles index');
let count = 0;
for (const p of probes.slice(chase)) {
  u16(count);
  count += p.bubbles.length;
}
u16(count);
end('bubbles index');

begin('bubbles');
for (const p of probes.slice(chase))
  for (const b of p.bubbles) {
    i16(b.x);
    i16(b.y);
    u8(b.scale);
  }
end('bubbles');

begin('shots');
u8(last.shots.length);
for (const s of last.shots) {
  u16(Math.round(s.fired * 1000));
  u16(s.hit ? Math.round(s.hit.at * 1000) : 65535);
  u8(s.hit ? ENEMY_NAMES.indexOf(s.hit.s) : 255);
  i16(s.mx);
  i16(s.my);
}
end('shots');

names.forEach((n, i) => {
  const v = sections[n]!;
  out[tableAt + i * 4] = v & 255;
  out[tableAt + i * 4 + 1] = (v >> 8) & 255;
  out[tableAt + i * 4 + 2] = (v >> 16) & 255;
  out[tableAt + i * 4 + 3] = (v >>> 24) & 255;
});

const print = process.argv.includes('--print');
console.error(`${TICKS} ticks, the chase in view from tick ${chase} (${(chase / 60).toFixed(3)} s); ${out.length} bytes`);
for (const n of names) console.error(`  ${n.padEnd(14)} ${String(sizes[n]).padStart(6)}`);
console.error(`${last.shots.length} shots, ${count} bubble sprites over the chase`);
for (const s of last.shots)
  console.error(`  fires ${s.fired.toFixed(3)} s at (${s.mx}, ${s.my}) ${s.hit ? `hits ${s.hit.s} at ${s.hit.at.toFixed(3)} s` : 'misses'}`);
if (!print) writeSync(1, Uint8Array.from(out));
