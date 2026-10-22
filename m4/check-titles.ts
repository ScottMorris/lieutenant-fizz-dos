// Compares frames of the opening titles with the oracle's, byte for byte.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//   bun m4/check-titles.ts --self                          the oracle's own frames: a checksum per tick and a sanity check
//   bun m4/check-titles.ts --dump titles.raw               every tick of the loop from the native bench (768 frames of 64000 bytes)
//   bun m4/check-titles.ts --raw m4/c/FRAME.RAW --tick N   one frame from the DOS build, as `FIZZ.EXE 255 N shot ... titles` writes it
//
// Exits non-zero if any byte differs, and for the first tick that does prints how many bytes and where the first one is.

import { readFileSync } from 'node:fs';
import { frame, SW, TICKS } from './titles/mock.ts';

const SIZE = 320 * 200;
const arg = (name: string) => {
  const i = process.argv.indexOf(name);
  return i < 0 ? undefined : process.argv[i + 1];
};

const checksum = (b: Uint8Array) => {
  let h = 0x811c9dc5;
  for (const v of b) h = Math.imul(h ^ v, 0x01000193) >>> 0;
  return h;
};

/** Counts the differing bytes of one frame and describes the first. */
function compare(n: number, got: Uint8Array, want: Uint8Array): string | undefined {
  let bad = 0;
  let first = -1;
  for (let i = 0; i < SIZE; i++)
    if (got[i] !== want[i]) {
      bad++;
      if (first < 0) first = i;
    }
  if (!bad) return undefined;
  return `tick ${n}: ${bad} bytes differ; the first at (${first % SW}, ${Math.floor(first / SW)}) is ${got[first]} and should be ${want[first]}`;
}

if (process.argv.includes('--self')) {
  const frames = Array.from({ length: TICKS }, (_, n) => frame(n));
  for (let n = 0; n < TICKS; n += 32) console.log(`tick ${String(n).padStart(3)}  ${checksum(frames[n]!).toString(16).padStart(8, '0')}`);
  const flash = frames.map((f, n) => (f.every((v) => v === 0xef) ? n : -1)).filter((n) => n >= 0);
  console.log(`white flash on ticks ${flash.join(', ')}`);
  // The loop's two ends are black, so it fades from nothing to nothing.
  const dark = (f: Uint8Array) => f.every((v) => v >> 4 === 1);
  if (!dark(frames[0]!) || flash.join() !== '265,266,267,268') {
    console.error('The oracle is not what the sequence expects: the first tick should be black and the flash on ticks 265 to 268');
    process.exit(1);
  }
  console.log('ok');
} else if (arg('--dump')) {
  const b = readFileSync(arg('--dump')!);
  if (b.length !== SIZE * TICKS) {
    console.error(`${arg('--dump')} is ${b.length} bytes; ${SIZE * TICKS} expected (${TICKS} frames)`);
    process.exit(1);
  }
  let failed = 0;
  for (let n = 0; n < TICKS; n++) {
    const msg = compare(n, b.subarray(n * SIZE, (n + 1) * SIZE), frame(n));
    if (msg && failed++ < 10) console.error(msg);
  }
  if (failed) {
    console.error(`${failed} of ${TICKS} ticks differ`);
    process.exit(1);
  }
  console.log(`all ${TICKS} ticks are identical to the oracle`);
} else if (arg('--raw') && arg('--tick')) {
  const n = Number(arg('--tick'));
  const b = readFileSync(arg('--raw')!);
  if (b.length < SIZE) {
    console.error(`${arg('--raw')} is ${b.length} bytes; ${SIZE} expected`);
    process.exit(1);
  }
  const msg = compare(n, b.subarray(0, SIZE), frame(n));
  if (msg) {
    console.error(msg);
    process.exit(1);
  }
  console.log(`tick ${n} is identical to the oracle`);
} else {
  console.error('usage: check-titles.ts --self | --dump <file> | --raw <FRAME.RAW> --tick <n>');
  process.exit(2);
}
