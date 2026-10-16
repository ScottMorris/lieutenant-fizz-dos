// Writes the DOS build's sound data: the OPL2 songs, the OPL2 and PC-speaker effects and the tables that choose them.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//   bun m4/export-sound.ts > m4/assets/sound.bin   (converts the songs and effects in m4/sound; reads the main repository and changes nothing in it)
//
// Layout (little endian):
//   "FZSN", u8 version 1, u8 songs, u8 effects, u8 captions, u8 levels
//   songs     × { u32 offset, u32 length }                      an IMF chunk with its 16-bit length
//   effects   × { u32 oplOffset, u16 oplLength, u32 speakerOffset, u16 speakerFrames }   IMF chunk; u16 timer divisors at 140 Hz
//   captions  × u8 effect number (255 for none), in the sim's caption order
//   levels    × u8 song number, in the sim's level order
//   u8 title song, u8 map song, u8 boss song
//   the data

import { readFileSync, writeSync } from 'node:fs';

const GAME = process.env.FIZZ_GAME ?? new URL('../../../liminal-hq/lieutenant-fizz', import.meta.url).pathname; // the main game's checkout; FIZZ_GAME overrides
const { PATTERNS } = await import('./sound/fizz-game/patterns.ts');
const { SONGS, planSong } = await import('./sound/fizz-songs.ts');
const { convertSong } = await import('./sound/fizz-dos/song.ts');
const { convertSfx } = await import('./sound/fizz-dos/sfx.ts');
const { LEVELS } = await import(`${GAME}/episodes/episode-1/src/story.ts`);

// The sim's own caption and level tables, read the way the web shell reads them.
const wasm = readFileSync(`${GAME}/episodes/episode-1/src/wasm/sim.wasm`);
const { instance } = await WebAssembly.instantiate(wasm, {});
const x = instance.exports as unknown as {
  memory: WebAssembly.Memory;
  table_len(t: number): number;
  name_ptr(t: number, i: number): number;
  name_len(t: number, i: number): number;
};
const captionNames: string[] = [];
for (let i = 0; i < x.table_len(1); i++)
  captionNames.push(new TextDecoder().decode(new Uint8Array(x.memory.buffer, x.name_ptr(1, i), x.name_len(1, i))));

const songIds: string[] = SONGS.map((d: { id: string }) => d.id);
const songs = SONGS.map((d: unknown) => convertSong(planSong(d)).imf as Uint8Array);
const effectNames = Object.keys(PATTERNS.sfx);
const effects = effectNames.map((n) => convertSfx(PATTERNS.sfx[n]));
const songOf = (track: string) => {
  const i = songIds.indexOf(track);
  if (i < 0) throw new Error(`No song for the track ${track}`);
  return i;
};
const captionFx = captionNames.map((c) => {
  const fx = PATTERNS.captionSfx[c];
  return fx === undefined ? 255 : effectNames.indexOf(fx);
});
const levelSong: number[] = LEVELS.map((l: { track: string }) => songOf(l.track));

const head = 4 + 1 + 4;
const songTable = songs.length * 8;
const fxTable = effects.length * 12;
const tail = captionFx.length + levelSong.length + 3;
let at = head + songTable + fxTable + tail;
const out: number[] = [];
const u8 = (v: number) => out.push(v & 255);
const u16 = (v: number) => (u8(v), u8(v >> 8));
const u32 = (v: number) => (u16(v), u16(v >> 16));
[...'FZSN'].forEach((c) => u8(c.charCodeAt(0)));
u8(1);
u8(songs.length);
u8(effects.length);
u8(captionFx.length);
u8(levelSong.length);
const data: Uint8Array[] = [];
for (const s of songs) {
  u32(at);
  u32(s.length);
  data.push(s);
  at += s.length;
}
for (const e of effects) {
  const spk = new Uint8Array(e.speaker.divisors.buffer, e.speaker.divisors.byteOffset, e.speaker.divisors.byteLength);
  u32(at);
  u16(e.opl.imf.length);
  data.push(e.opl.imf);
  at += e.opl.imf.length;
  u32(at);
  u16(e.speaker.divisors.length);
  data.push(spk);
  at += spk.length;
}
captionFx.forEach(u8);
levelSong.forEach(u8);
u8(songOf('title'));
u8(songOf('map'));
u8(songOf('boss'));
const total = Uint8Array.from(out);
const blob = new Uint8Array(at);
blob.set(total);
let p = total.length;
for (const d of data) (blob.set(d, p), (p += d.length));
writeSync(1, blob);
console.error(
  `${songs.length} songs, ${effects.length} effects, ${captionFx.length} captions (${captionFx.filter((v) => v !== 255).length} with a sound), ${levelSong.length} levels; ${blob.length} bytes`,
);
