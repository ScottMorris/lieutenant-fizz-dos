// The same run as src/main.rs, but through the web build's sim.wasm: `bun m0/hash.ts <path to sim.wasm>`.
import { readFileSync } from 'node:fs';

const wasm = readFileSync(process.argv[2] ?? '/home/scott/source/liminal-hq/lieutenant-fizz/episodes/episode-1/src/wasm/sim.wasm');
const { instance } = await WebAssembly.instantiate(wasm, {});
const x = instance.exports as unknown as {
  game_new(): void;
  enter_level(id: number): void;
  step(held: number): void;
  state_get(i: number): number;
};
const LEVELS = 16, TICKS = 3000, EVERY = 60, SLOTS = 26;
const view = new DataView(new ArrayBuffer(8));
const out: string[] = [];
for (let level = 0; level < LEVELS; level++) {
  x.game_new();
  x.enter_level(level);
  let s = (0x1234 + level) >>> 0;
  const next = () => (s = (Math.imul(s, 1664525) + 1013904223) >>> 0);
  let hold = 0, mask = 0;
  let h = 0xcbf29ce484222325n;
  for (let tick = 1; tick <= TICKS; tick++) {
    if (hold === 0) {
      const r = next();
      mask = (r >>> 16) & 0x7f;
      if (((r >>> 24) & 3) !== 0) { mask |= 2; mask &= ~1; }
      hold = 6 + ((r >>> 8) & 15);
    }
    hold--;
    x.step(mask);
    if (tick % EVERY === 0) {
      for (let i = 0; i < SLOTS; i++) {
        view.setFloat64(0, x.state_get(i), true);
        for (let b = 0; b < 8; b++) { h ^= BigInt(view.getUint8(b)); h = (h * 0x100000001b3n) & 0xffffffffffffffffn; }
      }
      out.push(`${level} ${tick} ${h.toString(16).padStart(16, '0')}`);
    }
  }
}
console.log(out.join('\n'));
