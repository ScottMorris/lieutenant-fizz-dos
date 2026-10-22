// Reads the pictures the opening titles are made of: the sprite sheet, the Fizz font, the strings and the Liminal HQ mark.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// The titles' oracle (`mock.ts`) and generator (`../export-titles.ts`) read the same files the DOS build links, so a picture here is a
// picture there: `m3/assets/sprites.bin` and `m2/dos-rt/src/ui_data.rs` (both generated from the main game), and `m4/assets/liminal-mark.txt`.

import { readFileSync } from 'node:fs';

const root = new URL('../../', import.meta.url).pathname;

/** The EGA colours' order, as the palette and the sheet index them: a code letter each (the handoff's codes). */
export const CODES = 'kBGCRMNLDbgcrmyW';
export const CI: Record<string, number> = Object.fromEntries([...CODES].map((c, i) => [c, i]));

export type Sprite = { w: number; h: number; /** an EGA colour 0..15, or 255 where the sprite is clear */ px: Uint8Array };

/** The sheet's names in id order: the sim's, then the cinematic's extras (`m3/export-sprites.ts`). */
function sheetNames(): string[] {
  const src = readFileSync(`${root}m2/game/src/sprites.rs`, 'utf8');
  const block = src.slice(src.indexOf('sprites! {'));
  const body = block.slice(0, block.indexOf('\n}\n'));
  const names = [...body.matchAll(/=\s*"([^"]+)"/g)].map((m) => m[1]!);
  const extra = ['planet', 'house', 'bigTree', 'moon', 'fence', 'labPanel', 'console0', 'console1', 'blueprint', 'bench', 'pad', 'lamp'];
  return [...names, ...extra];
}

export function loadSheet(): Map<string, Sprite> {
  const b = readFileSync(`${root}m3/assets/sprites.bin`);
  const v = new DataView(b.buffer, b.byteOffset, b.byteLength);
  const count = v.getUint16(0, true);
  const names = sheetNames();
  if (names.length !== count) throw new Error(`The sheet has ${count} sprites but sprites.rs names ${names.length}`);
  const sheet = new Map<string, Sprite>();
  for (let i = 0; i < count; i++) {
    const at = v.getUint32(2 + i * 4, true);
    const w = v.getUint16(at, true);
    const h = v.getUint16(at + 2, true);
    sheet.set(names[i]!, { w, h, px: new Uint8Array(b.buffer, b.byteOffset + at + 6, w * h).slice() });
  }
  return sheet;
}

export type Glyph = { w: number; rows: number[] };

/** The Fizz font and the title strings, from the generated `ui_data.rs`. */
export function loadUi(): { font: Map<number, Glyph>; kicker: string; words: string; line: string } {
  const src = readFileSync(`${root}m2/dos-rt/src/ui_data.rs`, 'utf8');
  const font = new Map<number, Glyph>();
  for (const m of src.matchAll(/Glyph \{ cp: (\d+), w: (\d+), rows: &\[([^\]]*)\] \}/g))
    font.set(Number(m[1]), { w: Number(m[2]), rows: m[3]!.split(',').map((x) => Number(x.trim())).filter((x) => !Number.isNaN(x)) });
  const str = (name: string) => src.match(new RegExp(`${name}: &str = "([^"]*)"`))![1]!;
  return { font, kicker: str('TITLE_KICKER'), words: str('TITLE_WORDS'), line: str('TITLE_LINE') };
}

/** The Liminal HQ mark: rows of two characters a pixel, a colour code and a brightness level in hex, `..` for clear. */
export function loadMark(): { w: number; h: number; p: string } {
  const rows = readFileSync(`${root}m4/assets/liminal-mark.txt`, 'utf8').trimEnd().split('\n');
  return { w: rows[0]!.length / 2, h: rows.length, p: rows.join('') };
}
