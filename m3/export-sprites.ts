// Writes the sprites the Episode 1 sim draws, in the sim's own id order, as 8-bit indexed images for the DOS blitter.
//   bun m3/export-sprites.ts > m3/assets/sprites.bin   (reads the main repository, changes nothing in it)
// Format (little endian): u16 count; count × u32 offsets from the start of the file; then per sprite u16 w, u16 h, u8 flags
// (1 = every pixel opaque, 2 = every pixel one colour), u8 that colour (when flag 2), and w*h bytes: 0 is transparent, 1..16 is the EGA colour 0..15 (the DOS palette keeps them in slots 1..16).
import { readFileSync, writeSync } from 'node:fs';
import { defineSprites } from '/home/scott/source/liminal-hq/lieutenant-fizz/episodes/episode-1/src/sprites/catalog';
import { EGA } from '/home/scott/source/liminal-hq/lieutenant-fizz/packages/engine/src/palette';

const wasm = readFileSync('/home/scott/source/liminal-hq/lieutenant-fizz/episodes/episode-1/src/wasm/sim.wasm');
const { instance } = await WebAssembly.instantiate(wasm, {});
const x = instance.exports as unknown as {
  memory: WebAssembly.Memory;
  table_len(t: number): number;
  name_ptr(t: number, i: number): number;
  name_len(t: number, i: number): number;
};
const defs = new Map(defineSprites().map((d) => [d.name, d.grid]));
const order = Object.keys(EGA);
const count = x.table_len(0);
const blobs: Uint8Array[] = [];
const missing: string[] = [];
for (let i = 0; i < count; i++) {
  const name = new TextDecoder().decode(new Uint8Array(x.memory.buffer, x.name_ptr(0, i), x.name_len(0, i)));
  const g = defs.get(name);
  if (!g) { missing.push(name); blobs.push(new Uint8Array([1, 0, 1, 0, 0, 0, 0])); continue; }
  const b = new Uint8Array(6 + g.w * g.h);
  new DataView(b.buffer).setUint16(0, g.w, true);
  new DataView(b.buffer).setUint16(2, g.h, true);
  for (let y = 0; y < g.h; y++)
    for (let xx = 0; xx < g.w; xx++) {
      const c = g.at(xx, y);
      b[6 + y * g.w + xx] = c ? order.indexOf(c) + 1 : 0;
    }
  const px = b.subarray(6);
  const opaque = px.every((v) => v !== 0);
  const uniform = opaque && px.every((v) => v === px[0]);
  b[4] = (opaque ? 1 : 0) | (uniform ? 2 : 0);
  b[5] = uniform ? px[0]! : 0;
  blobs.push(b);
}
const headLen = 2 + count * 4;
const out = new Uint8Array(headLen + blobs.reduce((s, b) => s + b.length, 0));
const dv = new DataView(out.buffer);
dv.setUint16(0, count, true);
let at = headLen;
blobs.forEach((b, i) => { dv.setUint32(2 + i * 4, at, true); out.set(b, at); at += b.length; });
writeSync(1, out);
console.error(`${count} sprites, ${out.length} bytes, missing: ${missing.join(', ') || 'none'}`);
