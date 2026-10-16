// Which sprites the sim draws for a level, how many of each, and how many pixels that is: bun m3/census.ts [level] [ticks]
import { readFileSync } from 'node:fs';
const wasm = readFileSync('/home/scott/source/liminal-hq/lieutenant-fizz/episodes/episode-1/src/wasm/sim.wasm');
const { instance } = await WebAssembly.instantiate(wasm, {});
const x = instance.exports as any;
const level = Number(process.argv[2] ?? 0), ticks = Number(process.argv[3] ?? 120);
x.game_new(); x.enter_level(level); x.set_view(10, 6.25);
const n = x.table_len(0);
const names: string[] = [], sizes: [number, number][] = [];
const bin = readFileSync(new URL('./assets/sprites.bin', import.meta.url));
const bv = new DataView(bin.buffer, bin.byteOffset);
for (let i = 0; i < n; i++) {
  names.push(new TextDecoder().decode(new Uint8Array(x.memory.buffer, x.name_ptr(0, i), x.name_len(0, i))));
  const off = bv.getUint32(2 + i * 4, true);
  sizes.push([bv.getUint16(off, true), bv.getUint16(off + 2, true)]);
}
const tab = new Float32Array(x.memory.buffer, x.sprite_table_ptr(), n * 6);
for (let i = 0; i < n; i++) tab.set([i, 0, 1, 1, sizes[i]![0], sizes[i]![1]], i * 6);
for (let t = 0; t < ticks; t++) x.step(2);
const count = x.render(1, 0);
const inst = new Float32Array(x.memory.buffer, x.instance_ptr(), count * 20);
const camX = new Float32Array(x.memory.buffer, x.out_ptr(), 32)[1]!, camY = new Float32Array(x.memory.buffer, x.out_ptr(), 32)[2]!;
const by = new Map<string, { n: number; px: number; vis: number }>();
let total = 0, visible = 0;
for (let i = 0; i < count; i++) {
  const b = inst.subarray(i * 20, i * 20 + 20);
  const id = b[16]!, nm = names[id]!;
  const w = Math.abs(b[0]!) * 16, h = Math.abs(b[5]!) * 16;
  const cx = 160 + (b[12]! - camX) * 16, cy = 100 - (b[13]! - camY) * 16;
  const ox = Math.max(0, Math.min(320, cx + w / 2) - Math.max(0, cx - w / 2)), oy = Math.max(0, Math.min(200, cy + h / 2) - Math.max(0, cy - h / 2));
  const e = by.get(nm) ?? { n: 0, px: 0, vis: 0 }; e.n++; e.px += w * h; e.vis += ox * oy; by.set(nm, e);
  total += w * h; visible += ox * oy;
}
console.log(`${count} instances; ${Math.round(total)} px of sprite area, ${Math.round(visible)} of it on the 320x200 screen`);
[...by.entries()].sort((a, b) => b[1].vis - a[1].vis).slice(0, 14).forEach(([k, v]) => console.log(String(v.n).padStart(5), k.padEnd(14), String(Math.round(v.vis)).padStart(7), 'px on screen'));
