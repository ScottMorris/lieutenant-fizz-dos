// Reads Lieutenant Fizz music into timed events, using Undertone's own pattern parser
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: MIT

import * as U from '@liminal-hq/undertone';
import { Fraction, noteToFrequency, type Hap, type SoundType } from '@liminal-hq/undertone';

import type { MusicPart, MusicTrack } from './fizz-game/types.js';

export type SheetEvent = {
	/** Which part of the track, in the order the game lists them. */
	part: number;
	/** Start and end in cycles from the start of the loop. */
	t0: number;
	t1: number;
	/** Pitch in Hz, or undefined for noise. */
	hz?: number;
	noise?: 'white' | 'pink' | 'brown';
};

export type SheetTrack = {
	events: SheetEvent[];
	/** Cycles in the loop: the shortest length after which every part repeats. */
	cycles: number;
	/** Length of the loop in seconds (a cycle is four beats). */
	seconds: number;
};

const cyclesOf = (fraction: Fraction) => fraction.n / fraction.d;

function partPattern(part: MusicPart) {
	return part.noise ? U.sound(part.notes as SoundType) : U.note(part.notes);
}

/** Events of one part for cycles [from, to), onsets only. */
export function queryPart(part: MusicPart, index: number, from: number, to: number): SheetEvent[] {
	const pattern = partPattern(part);
	const events: SheetEvent[] = [];
	for (let cycle = from; cycle < to; cycle++) {
		pattern
			.query({ begin: new Fraction(cycle), end: new Fraction(cycle + 1) })
			.forEach((hap: Hap<any>) => {
				if (!hap.whole || cyclesOf(hap.part.begin) !== cyclesOf(hap.whole.begin)) return;
				const value = hap.value;
				const isNoise =
					value.soundType === 'white' || value.soundType === 'pink' || value.soundType === 'brown';
				events.push({
					part: index,
					t0: cyclesOf(hap.whole.begin),
					t1: cyclesOf(hap.whole.end),
					hz: isNoise || value.pitch === undefined ? undefined : noteToFrequency(value.pitch),
					noise: isNoise ? value.soundType : undefined,
				});
			});
	}
	return events;
}

const signature = (events: SheetEvent[], cycle: number) =>
	events
		.filter((event) => Math.floor(event.t0 + 1e-9) === cycle)
		.map(
			(event) =>
				`${event.part}:${(event.t0 - cycle).toFixed(4)}:${(event.t1 - event.t0).toFixed(4)}:${event.hz?.toFixed(1) ?? event.noise}`,
		)
		.sort()
		.join('|');

/** The events of one loop of a track. */
export function readTrack(track: MusicTrack): SheetTrack {
	const horizon = 32;
	const all = track.parts.flatMap((part, index) => queryPart(part, index, 0, horizon));
	const signatures = Array.from({ length: horizon }, (_, cycle) => signature(all, cycle));
	let cycles = horizon;
	for (const candidate of [1, 2, 3, 4, 6, 8, 12, 16]) {
		let repeats = true;
		for (let cycle = 0; cycle + candidate < horizon && repeats; cycle++) {
			repeats = signatures[cycle] === signatures[cycle + candidate];
		}
		if (repeats) {
			cycles = candidate;
			break;
		}
	}
	return {
		events: all.filter((event) => event.t0 < cycles),
		cycles,
		seconds: (cycles * 240) / track.bpm,
	};
}
