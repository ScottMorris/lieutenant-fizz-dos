// The OPL2 instruments the Fizz DOS conversions use, chosen by what a part does and the wave the game gives it
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: MIT

import type { OscillatorType, PartRole } from '../fizz-game/types.js';
import type { Operator, Patch } from './opl.js';

const op = (o: Partial<Operator> & Pick<Operator, 'mult' | 'tl'>): Operator => ({
	ar: 15,
	dr: 4,
	sl: 3,
	rr: 7,
	sustain: true,
	...o,
});

/**
 * The patches. The game's square, triangle, sawtooth and sine become what a two-operator FM voice can say: the
 * modulator's level and feedback shape the brightness (feedback 5 to 7 is the buzzy, saw-like edge), a sine carrier
 * alone is the soft triangle/sine, and a percussive envelope (no sustain) is a pluck or bell. These are first guesses
 * to be tuned by ear.
 */
export const PATCHES = {
	leadSquare: {
		name: 'lead (square)',
		mod: op({ mult: 1, tl: 24, dr: 4, sl: 4 }),
		car: op({ mult: 1, tl: 0, wave: 0 }),
		feedback: 4,
	},
	leadSaw: {
		name: 'lead (saw)',
		mod: op({ mult: 1, tl: 14 }),
		car: op({ mult: 1, tl: 0 }),
		feedback: 7,
	},
	leadSoft: {
		name: 'lead (triangle/sine)',
		mod: op({ mult: 1, tl: 40 }),
		car: op({ mult: 1, tl: 0 }),
		feedback: 2,
	},
	bass: {
		name: 'bass',
		mod: op({ mult: 1, tl: 34, dr: 6, sl: 6 }),
		car: op({ mult: 1, tl: 0, dr: 5, sl: 3, rr: 8 }),
		feedback: 2,
	},
	bassSaw: {
		name: 'bass (saw/square)',
		mod: op({ mult: 1, tl: 20, dr: 6, sl: 5 }),
		car: op({ mult: 1, tl: 0, dr: 5, sl: 3, rr: 8 }),
		feedback: 6,
	},
	pad: {
		name: 'pad',
		mod: op({ mult: 1, tl: 38, ar: 6, dr: 3 }),
		car: op({ mult: 1, tl: 4, ar: 6, dr: 3, sl: 2, rr: 5 }),
		feedback: 2,
	},
	padBright: {
		name: 'pad (square/saw)',
		mod: op({ mult: 2, tl: 30, ar: 7, dr: 3 }),
		car: op({ mult: 1, tl: 4, ar: 7, dr: 3, sl: 2, rr: 5 }),
		feedback: 4,
	},
	pluck: {
		name: 'pluck',
		mod: op({ mult: 2, tl: 26, ar: 15, dr: 10, sl: 12, sustain: false }),
		car: op({ mult: 1, tl: 0, ar: 15, dr: 9, sl: 8, rr: 8, sustain: false }),
		feedback: 3,
	},
	bell: {
		name: 'bell',
		mod: op({ mult: 7, tl: 28, ar: 15, dr: 7, sl: 15, sustain: false }),
		car: op({ mult: 1, tl: 0, ar: 15, dr: 6, sl: 10, rr: 6, sustain: false }),
		feedback: 0,
	},
	drone: {
		name: 'drone',
		mod: op({ mult: 1, tl: 22, ar: 8, dr: 3 }),
		car: op({ mult: 1, tl: 2, ar: 8, dr: 3, sl: 2, rr: 5 }),
		feedback: 7,
	},
	/** A noisy buzz for the effects' noise bursts: high feedback at a jittering pitch. */
	noise: {
		name: 'noise',
		mod: op({ mult: 1, tl: 8, ar: 15, dr: 6, sl: 8 }),
		car: op({ mult: 1, tl: 0, ar: 15, dr: 6, sl: 8, rr: 8 }),
		feedback: 7,
	},
	/** A held, clean tone for the effects: the carrier does the work and the level is moved a frame at a time. */
	fxSquare: {
		name: 'effect (square)',
		mod: op({ mult: 1, tl: 26, ar: 15, dr: 0, sl: 0, rr: 12 }),
		car: op({ mult: 1, tl: 0, ar: 15, dr: 0, sl: 0, rr: 12 }),
		feedback: 4,
	},
	fxSaw: {
		name: 'effect (saw)',
		mod: op({ mult: 1, tl: 16, ar: 15, dr: 0, sl: 0, rr: 12 }),
		car: op({ mult: 1, tl: 0, ar: 15, dr: 0, sl: 0, rr: 12 }),
		feedback: 7,
	},
	fxSoft: {
		name: 'effect (triangle/sine)',
		mod: op({ mult: 1, tl: 44, ar: 15, dr: 0, sl: 0, rr: 12 }),
		car: op({ mult: 1, tl: 0, ar: 15, dr: 0, sl: 0, rr: 12 }),
		feedback: 1,
	},
} satisfies Record<string, Patch>;

export type PatchName = keyof typeof PATCHES;

/** The patch for a music part, from its role and the oscillator the game gave it. */
export function patchForPart(role: PartRole | undefined, wave: OscillatorType | undefined): Patch {
	const w = wave ?? 'triangle';
	const edgy = w === 'square' || w === 'sawtooth';
	switch (role) {
		case 'lead':
		case 'counter':
			return w === 'sawtooth' ? PATCHES.leadSaw : edgy ? PATCHES.leadSquare : PATCHES.leadSoft;
		case 'bass':
			return edgy ? PATCHES.bassSaw : PATCHES.bass;
		case 'pad':
			return edgy ? PATCHES.padBright : PATCHES.pad;
		case 'arp':
			return PATCHES.pluck;
		case 'bell':
			return PATCHES.bell;
		case 'drone':
			return PATCHES.drone;
		default:
			return edgy ? PATCHES.leadSquare : PATCHES.leadSoft;
	}
}

/** The effect patch for a voice's oscillator. */
export function patchForEffect(wave: OscillatorType | undefined): Patch {
	const w = wave ?? 'triangle';
	return w === 'square' ? PATCHES.fxSquare : w === 'sawtooth' ? PATCHES.fxSaw : PATCHES.fxSoft;
}

/**
 * The OPL2's rhythm mode: five drums on channels 6 to 8 (bass drum on channel 6, hi-hat and snare on 7, tom and cymbal
 * on 8). Each drum has its own operator and the noise comes from the chip. `hz` is the frequency of the channel the
 * drum sits on, which tunes it.
 */
export const DRUM_CHANNEL_HZ = [150, 800, 300] as const;
export const DRUMS = {
	bass: { bit: 0x10, holdTicks: 40, ops: [0x10, 0x13] as const },
	snare: { bit: 0x08, holdTicks: 30, ops: [0x14] as const },
	tom: { bit: 0x04, holdTicks: 40, ops: [0x12] as const },
	cymbal: { bit: 0x02, holdTicks: 60, ops: [0x15] as const },
	hat: { bit: 0x01, holdTicks: 14, ops: [0x11] as const },
} as const;

export type DrumName = keyof typeof DRUMS;

/** How each drum's operators are set (the bass drum has two: a modulator and a carrier). */
export const DRUM_PATCHES: Record<DrumName, Operator[]> = {
	bass: [
		op({ mult: 0, tl: 26, ar: 15, dr: 9, sl: 15, rr: 7, sustain: false }),
		op({ mult: 0, tl: 6, ar: 15, dr: 8, sl: 15, rr: 7, sustain: false }),
	],
	snare: [op({ mult: 1, tl: 12, ar: 15, dr: 10, sl: 15, rr: 8, sustain: false })],
	tom: [op({ mult: 1, tl: 12, ar: 15, dr: 9, sl: 15, rr: 8, sustain: false })],
	cymbal: [op({ mult: 1, tl: 20, ar: 15, dr: 9, sl: 15, rr: 6, sustain: false })],
	hat: [op({ mult: 1, tl: 22, ar: 15, dr: 12, sl: 15, rr: 9, sustain: false })],
};

/** Which drum a part plays. */
export function drumForRole(role: PartRole | undefined): DrumName | undefined {
	switch (role) {
		case 'kick':
			return 'bass';
		case 'snare':
			return 'snare';
		case 'hats':
			return 'hat';
		case 'perc':
			return 'cymbal';
		default:
			return undefined;
	}
}
