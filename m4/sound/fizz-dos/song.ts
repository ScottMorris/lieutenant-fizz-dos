// Converts a Lieutenant Fizz song (the loop grown to about a minute) into an OPL2 IMF stream for the DOS build
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: MIT

import { queryPart, type SheetEvent } from '../fizz-sheet-events.js';
import type { MusicPart } from '../fizz-game/types.js';
import type { Plan } from '../fizz-songs.js';
import { convertParts, type MusicConversion } from './music.js';

export type SongConversion = MusicConversion & {
	/** The sections in order, with the second each begins at. */
	sections: { name: string; at: number }[];
};

/**
 * Turns a planned song into one IMF stream. Each section's slots are queried from the section's own first bar, as the
 * arranger plays them, and laid end to end; a note is cut at the end of its section. A slot that appears in several
 * sections keeps the one channel preference, because it is one part to the converter.
 */
export function convertSong(plan: Plan): SongConversion {
	const trim = plan.def.trim;
	const keys = [
		...new Set(plan.sections.flatMap((section) => section.slots.map((slot) => slot.key))),
	];
	const parts: MusicPart[] = [];
	const events: SheetEvent[] = [];
	const marks: { name: string; at: number }[] = [];
	let start = 0;
	for (const section of plan.sections) {
		marks.push({ name: section.name, at: (start * 240) / plan.bpm });
		for (const slot of section.slots) {
			const index = keys.indexOf(slot.key);
			// The first appearance of a slot sets its part for the converter (role, wave, gain); later ones differ only
			// in notes (and, for a lifted tune, pitch).
			parts[index] ??= { ...slot.part, g: slot.part.g * trim };
			for (const e of queryPart(slot.part, index, 0, section.cycles)) {
				events.push({
					...e,
					t0: start + e.t0,
					t1: start + Math.min(e.t1, section.cycles),
				});
			}
		}
		start += section.cycles;
	}
	return { ...convertParts(parts, events, plan.bpm, start), sections: marks };
}
