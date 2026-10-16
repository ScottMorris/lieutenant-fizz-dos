// Builds one Undertone voice from Lieutenant Fizz voice data, as the game does
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: MIT

// Mirrors buildVoice in packages/engine/src/audio.ts. Music parts hold their envelope for the
// note length (gated); effects are percussive.

import type { ControlPatch, Pattern, SoundType } from '@liminal-hq/undertone';

import { partMakeup, undertonePan } from './field.js';
import type { MusicPart, SfxVoice } from './types.js';

type UndertoneModule = typeof import('@liminal-hq/undertone');
type Voice = Pattern<ControlPatch>;

const isNoise = (text: string) => text === 'white' || text === 'pink' || text === 'brown';

export function buildVoice(
	U: UndertoneModule,
	v: SfxVoice | MusicPart,
	volume: number,
	gated: boolean,
	placed = false,
	pan?: number,
): Voice {
	const text = 'notes' in v ? v.notes : v.n;
	const role = placed && 'notes' in v ? v.role : undefined;
	const noisy = 'noise' in v ? !!v.noise : isNoise(text);
	let p: Voice = noisy
		? U.sound(text as SoundType)
		: U.note(text).sound((v.w ?? 'triangle') as SoundType);
	p = p
		.attack(v.a ?? 0.001)
		.decay(v.d ?? 0.1)
		.sustain(gated ? ('s' in v ? (v.s ?? 0.3) : 0.3) : 0)
		.release(v.r ?? 0.05)
		.gain(v.g * volume * partMakeup(role));
	const rolePan = undertonePan(role);
	if (rolePan !== undefined) p = p.pan(rolePan);
	else if (pan !== undefined && pan !== 0) p = p.pan(pan);
	if (v.lpf) p = p.lpf(v.lpf);
	if (v.hpf) p = p.hpf(v.hpf);
	if (v.slide) p = p.slide(v.slide);
	if ('nudge' in v && v.nudge) p = p.nudge(v.nudge);
	if ('room' in v && v.room) p = p.room(v.room).roomsize(6).orbit(1);
	if ('delay' in v && v.delay) p = p.delay(v.delay).delaytime(0.33).delayfeedback(0.35).orbit(2);
	return p;
}
