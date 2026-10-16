// The OPL2's shaping ideas applied to Undertone voices, without FM
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: MIT

import * as U from '@liminal-hq/undertone';

import { PART_PAN } from './fizz-game/field.js';
import type { MusicPart, PartRole } from './fizz-game/types.js';
import { buildVoice } from './fizz-game/voices.js';

const PERCUSSIVE: ReadonlySet<PartRole> = new Set(['kick', 'snare', 'hats', 'perc']);

/** The pan for the partner layer: the other side of its part. */
function oppositePan(part: MusicPart, index: number): number | string {
	const pan = part.role ? PART_PAN[part.role] : 0;
	if (typeof pan === 'number') return pan === 0 ? (index % 2 === 0 ? -0.22 : 0.22) : -pan;
	return `[${pan[1]} ${pan[0]}]*4`;
}

/**
 * Applies what the OPL2 does without FM, using only Undertone's existing controls:
 * a filter envelope that opens at the attack and closes as the note settles (the audible
 * effect of a decaying modulator), and a partner voice on the same note, a few ticks late, with
 * a different wave, more sustain and the opposite pan (the layering found in the Keen cues).
 */
export function depthVoices(part: MusicPart, index: number, placed: boolean) {
	if (part.noise || (part.role && PERCUSSIVE.has(part.role))) {
		return [buildVoice(U, part, 1, true, placed)];
	}
	const slow = part.role === 'pad' || part.role === 'drone';
	const cutoff = part.lpf ?? 2600;
	const settle = slow ? 1 : Math.min(0.4, Math.max(0.08, (part.d ?? 0.12) * 1.5));
	const shaped = (voice: ReturnType<typeof buildVoice>, open: number, close: number) =>
		voice
			.lpf(cutoff)
			.lpenv(open)
			.lpa(slow ? 0.3 : 0.001)
			.lpd(close)
			.lps(0.2)
			.lpr(part.r ?? 0.08);
	const main = shaped(
		buildVoice(U, { ...part, g: part.g * (slow ? 1 : 0.85) }, 1, true, placed),
		cutoff * (slow ? 0.8 : 1.4),
		settle,
	);
	if (slow) return [main];
	const partner = buildVoice(
		U,
		{
			...part,
			w: 'sine',
			g: part.g * 0.5,
			a: Math.max(part.a ?? 0.005, 0.01),
			s: Math.max(part.s ?? 0.3, 0.5),
			lpf: 3200,
		},
		1,
		true,
		placed,
	).nudge(0.002);
	return [main, placed ? partner.pan(oppositePan(part, index)) : partner];
}
