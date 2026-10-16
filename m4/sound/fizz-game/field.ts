// Where each music role sits in the stereo field, as in Lieutenant Fizz's Enhanced sound
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: MIT

// Mirrors PART_PAN, ARP_STEPS and PANNED_MAKEUP in packages/engine/src/sound-field.ts.

import type { PartRole } from './types.js';

export type PartPan = number | readonly [number, number];

/** Low and central roles stay in the middle; the rest lean a little, never past 0.3. */
export const PART_PAN: Record<PartRole, PartPan> = {
	bass: 0,
	kick: 0,
	snare: 0,
	pad: 0,
	drone: 0,
	lead: -0.15,
	counter: 0.2,
	bell: 0.3,
	hats: 0.25,
	perc: -0.3,
	arp: [-0.3, 0.3],
};

/** The arpeggio alternates sides on every eighth of the cycle. */
export const ARP_STEPS = 8;

/** A panned part needs a little extra gain to keep its level. */
export const PANNED_MAKEUP = Math.SQRT2;

export const isPanned = (role: PartRole | undefined) => role !== undefined && PART_PAN[role] !== 0;

export const partMakeup = (role: PartRole | undefined) => (isPanned(role) ? PANNED_MAKEUP : 1);

/** The pan of a part at `cyclePos` (0 up to 1 through its cycle). */
export function partPanAt(role: PartRole | undefined, cyclePos: number): number {
	if (!role) return 0;
	const pan = PART_PAN[role];
	if (typeof pan === 'number') return pan;
	const step = Math.floor(cyclePos * ARP_STEPS + 1e-9);
	return pan[((step % 2) + 2) % 2]!;
}

/** The value for Undertone's `.pan()`: a number, an alternating pattern, or none. */
export function undertonePan(role: PartRole | undefined): number | string | undefined {
	if (!role || !isPanned(role)) return undefined;
	const pan = PART_PAN[role];
	return typeof pan === 'number' ? pan : `[${pan[0]} ${pan[1]}]*${ARP_STEPS / 2}`;
}
