// Extends short Lieutenant Fizz loops into full songs with sections, using common song-writing devices
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: MIT

import * as U from '@liminal-hq/undertone';
import type { ControlPatch, Pattern } from '@liminal-hq/undertone';

import { depthVoices } from './fizz-depth.js';
import { PATTERNS } from './fizz-game/patterns.js';
import { trackNotes } from './fizz-game/track-notes.js';
import type { MusicPart, MusicTrack } from './fizz-game/types.js';
import { buildVoice } from './fizz-game/voices.js';
import { readTrack } from './fizz-sheet-events.js';
import { AIRE, FUNK, GROOVE, SQUEEZE, TIPTOE } from './fizz-songs-originals.js';

type Voice = Pattern<ControlPatch>;

// ---------- Rewriting notes ----------

const SHARP_NAMES = ['c', 'c#', 'd', 'd#', 'e', 'f', 'f#', 'g', 'g#', 'a', 'a#', 'b'];
const SEMITONES: Record<string, number> = { c: 0, d: 2, e: 4, f: 5, g: 7, a: 9, b: 11 };
const NOTE = /(?<![a-z#])([a-g])(#|b)?(-?\d)(?!\d)/g;

/** Shifts every note name in a mini-notation string by `semitones`. Noise words are untouched. */
export function transposeNotes(notes: string, semitones: number): string {
	if (!semitones) return notes;
	return notes.replace(
		NOTE,
		(_, letter: string, accidental: string | undefined, octave: string) => {
			const midi =
				12 * (Number(octave) + 1) +
				SEMITONES[letter] +
				(accidental === '#' ? 1 : accidental === 'b' ? -1 : 0) +
				semitones;
			return `${SHARP_NAMES[((midi % 12) + 12) % 12]}${Math.floor(midi / 12) - 1}`;
		},
	);
}

/** A bass line reduced to the first note of each bar: the root, held or repeated. */
export function rootsOnly(notes: string): string {
	return notes.replace(/\[([^[\],]*)\]/g, (_, group: string) => group.trim().split(/\s+/)[0]);
}

/** Block chords `[a,b,c]` spread into a rising and falling arpeggio of eight notes a bar. */
export function arpeggiate(notes: string): string {
	return notes.replace(/\[([^[\]]+,[^[\]]+)\]/g, (_, group: string) => {
		const tones = group.split(',').map((tone) => tone.trim());
		const up = [...tones, ...tones.slice(1, -1).reverse()];
		return `[${Array.from({ length: 8 }, (_, step) => up[step % up.length]).join(' ')}]`;
	});
}

// ---------- Sections ----------

type Slot = { key: string; part: MusicPart };
export type Section = { name: string; cycles: number; what: string; slots: Slot[] };

const SOFT: ReadonlySet<string> = new Set(['pad', 'drone', 'bell']);

const melodic = (part: MusicPart) => !part.noise && part.role !== 'kick';
const moved = (part: MusicPart, shift: number): MusicPart =>
	melodic(part) ? { ...part, notes: transposeNotes(part.notes, shift) } : part;

/** The part that carries the tune: a lead if there is one, else an arpeggio, a bell or a counter. */
function primary(track: MusicTrack): number {
	for (const role of ['lead', 'arp', 'bell', 'counter']) {
		const index = track.parts.findIndex((part) => part.role === role);
		if (index >= 0) return index;
	}
	return 0;
}

const slots = (
	track: MusicTrack,
	pick: (part: MusicPart, index: number) => MusicPart | undefined,
) =>
	track.parts.flatMap((part, index): Slot[] => {
		const chosen = pick(part, index);
		return chosen ? [{ key: `p${index}`, part: chosen }] : [];
	});

const arpOf = (track: MusicTrack, shift: number): Slot[] => {
	const pad = track.parts.find((part) => part.role === 'pad');
	if (!pad || !/\[[^[\]]+,[^[\]]+\]/.test(pad.notes)) return [];
	return [
		{
			key: 'arp',
			part: {
				...pad,
				notes: transposeNotes(arpeggiate(pad.notes), shift),
				role: 'arp',
				w: 'triangle',
				a: 0.005,
				d: 0.1,
				s: 0.2,
				r: 0.1,
				g: 0.06,
				lpf: 2400,
				room: 0.3,
			},
		},
	];
};

const roll = (g: number): Slot => ({
	key: 'roll',
	part: { notes: 'white*16', role: 'snare', noise: true, d: 0.05, s: 0, r: 0.03, g, hpf: 1500 },
});

const fourOnFloor: Slot = {
	key: 'floor',
	part: { notes: 'c1*4', role: 'kick', w: 'sine', d: 0.12, s: 0, r: 0.06, g: 0.6, slide: 0.09 },
};

/** The cast only: pad or bell, a bass reduced to roots, and the hats. */
function intro(track: MusicTrack, cycles: number): Section {
	return {
		name: 'Intro',
		cycles,
		what: 'pads and hats with the bass on its roots, so the groove arrives gradually',
		slots: slots(track, (part) => {
			if (part.role && SOFT.has(part.role)) return part;
			if (part.role === 'bass') return { ...part, notes: rootsOnly(part.notes), s: 0.7 };
			if (part.role === 'hats') return part;
			return undefined;
		}),
	};
}

type MainOptions = { shift?: number; arp?: boolean; octave?: boolean };

/** The full band, optionally in another key, with the pad arpeggiated or the tune doubled an octave up. */
function main(
	track: MusicTrack,
	cycles: number,
	name: string,
	what: string,
	{ shift = 0, arp = false, octave = false }: MainOptions,
): Section {
	const lead = primary(track);
	const list = slots(track, (part) => moved(part, shift));
	if (arp) list.push(...arpOf(track, shift));
	if (octave) {
		const part = track.parts[lead];
		list.push({
			key: 'octave',
			part: { ...part, notes: transposeNotes(part.notes, shift + 12), g: part.g * 0.55 },
		});
	}
	return { name, cycles, what, slots: list };
}

/** Everything drops out but the pad, the roots and a kick. */
function breakdown(track: MusicTrack, cycles: number, build: boolean): Section {
	const base = slots(track, (part) => {
		if (part.role && SOFT.has(part.role)) return part;
		// Held roots carry a lot of low energy, so the bass and kick drop back in a breakdown.
		if (part.role === 'bass')
			return { ...part, notes: rootsOnly(part.notes), s: 0.5, g: part.g * 0.5 };
		if (part.role === 'kick') return { ...part, g: part.g * 0.6 };
		return undefined;
	});
	return {
		name: build ? 'Build' : 'Breakdown',
		cycles,
		what: build
			? 'a snare roll under the held pad, running into the last chorus'
			: 'the band drops out, leaving the pad, the roots and a kick',
		slots: build ? [...base, roll(0.2)] : base,
	};
}

/** One bar that turns the end of the song back into its beginning. */
function turnaround(track: MusicTrack, level = 1): Section {
	const pad = track.parts.find((part) => part.role && SOFT.has(part.role));
	const bass = track.parts.find((part) => part.role === 'bass');
	return {
		name: 'Fill',
		cycles: 1,
		what: 'a snare roll and four kicks over a held pad and root, so the loop lands on the intro with no seam',
		slots: [
			roll(0.22 * level),
			{ ...fourOnFloor, part: { ...fourOnFloor.part, g: fourOnFloor.part.g * level } },
			...(pad ? [{ key: 'pad', part: pad }] : []),
			...(bass
				? [
						{
							key: 'bass',
							part: { ...bass, notes: rootsOnly(bass.notes), s: 0.7, g: bass.g * 0.8 },
						},
					]
				: []),
		],
	};
}

/** A section laid out by hand: the named roles play, in the given key. */
function layered(track: MusicTrack, step: BuildStep): Section {
	const chosen = track.parts.flatMap((part, index): Slot[] => {
		const role = part.role ?? 'melody';
		if (step.roles !== 'all' && !step.roles.includes(role)) return [];
		return [{ key: `p${index}`, part: moved(part, step.shift ?? 0) }];
	});
	if (step.octave) {
		const lead = primary(track);
		const part = track.parts[lead];
		const role = part.role ?? 'melody';
		if (step.roles === 'all' || step.roles.includes(role)) {
			chosen.push({
				key: 'octave',
				part: {
					...part,
					notes: transposeNotes(part.notes, (step.shift ?? 0) + 12),
					g: part.g * 0.55,
				},
			});
		}
	}
	return { name: step.name, cycles: step.cycles, what: step.what, slots: chosen };
}

// ---------- Songs ----------

/** Where a song comes from, for the page. */
export type Origin =
	| { kind: 'extends'; cue: string; name: string }
	| { kind: 'inspired'; game: string; title: string; chunk: number; facts: string }
	| { kind: 'original' };

/** One step of a song laid out by hand: which roles play, in what key. */
export type BuildStep = {
	name: string;
	cycles: number;
	/** The roles that play, or every part. */
	roles: string[] | 'all';
	shift?: number;
	octave?: boolean;
	what: string;
	/** A breakdown or a build, which the arranger writes itself. */
	kind?: 'breakdown' | 'build';
};

export type SongDef = {
	id: string;
	title: string;
	style: string;
	track: MusicTrack;
	/** Where the contrast section goes: semitones above the main key. */
	contrast: number;
	/** Where the last chorus goes. */
	lift: number;
	arp: boolean;
	/** The song-writing devices used, for the page. */
	devices: string[];
	/**
	 * A level trim that keeps the summed peak under about 0.9. Undertone writes straight to the
	 * output with no limiter, and offline renders of the full songs peaked at 1.0 to 1.2.
	 */
	trim: number;
	origin: Origin;
	/** How loud the closing fill is against the song: a sparse song needs a gentler one. */
	fill?: number;
	/** Bars of intro before the first main section; 0 starts straight on the tune. Default 2. */
	intro?: number;
	/** Leave out the closing fill, so the last section runs straight into the first. */
	noFill?: boolean;
	/** Pin the form to one of the named forms instead of choosing the one nearest a minute. */
	form?: string;
	/** A hand-laid form (layers arriving section by section) instead of a chosen one. */
	build?: BuildStep[];
};

const fromGame = (
	id: string,
	_title: string,
): { track: MusicTrack; style: string; origin: Origin } => ({
	track: PATTERNS.music[id],
	style: trackNotes[id] ?? '',
	origin: { kind: 'extends', cue: id, name: (trackNotes[id] ?? id).split(':')[0] },
});

export const SONGS: SongDef[] = [
	{
		id: 'title',
		trim: 0.75,
		intro: 0,
		noFill: true,
		title: 'Title Theme',
		...fromGame('title', 'Title Theme'),
		contrast: 5,
		lift: 0,
		arp: true,
		devices: [
			'contrast section in the subdominant',
			'pad arpeggiated for the second chorus',
			'no intro and no fill: the last tune, in the home key, runs straight back into the first',
			'the arrangement the game and its launcher play on the title and App settings',
		],
	},
	{
		id: 'crater',
		trim: 0.75,
		title: 'Crater Run',
		...fromGame('crater', 'Crater Run'),
		contrast: 5,
		lift: 2,
		arp: true,
		devices: [
			'contrast section in the subdominant',
			'pad arpeggiated for the second chorus',
			'final key lift',
		],
	},
	{
		id: 'caves',
		trim: 1,
		title: 'Dripstone Echoes',
		...fromGame('caves', 'Dripstone Echoes'),
		contrast: 7,
		lift: 0,
		arp: true,
		devices: [
			'contrast section in the dominant',
			'breakdown to a held pad and roots',
			'no key lift: the cave stays eerie',
		],
	},
	{
		id: 'sky',
		trim: 0.88,
		title: 'Open Sky',
		...fromGame('sky', 'Open Sky'),
		contrast: 7,
		lift: 2,
		arp: true,
		devices: ['verse to chorus up a fifth', 'octave doubling on the return', 'final key lift'],
	},
	{
		id: 'tower',
		trim: 0.92,
		title: 'Climb the Tower',
		...fromGame('tower', 'Climb the Tower'),
		contrast: 2,
		lift: 5,
		arp: false,
		devices: [
			'the ostinato steps up a tone, then a fourth, so it never stops rising',
			'breakdown and snare-roll build',
		],
	},
	{
		id: 'theatre',
		trim: 1,
		title: 'Theatre Waltz',
		...fromGame('theatre', 'Theatre Waltz'),
		contrast: 5,
		lift: 0,
		arp: false,
		devices: [
			'waltz in the old three-part form: tune, trio in the subdominant, tune again',
			'bass held on the root for the breakdown',
		],
	},
	{
		id: 'foundry',
		trim: 0.74,
		title: 'Foundry Floor',
		...fromGame('foundry', 'Foundry Floor'),
		contrast: 5,
		lift: 0,
		arp: false,
		devices: [
			'mechanical groove that stays put',
			'breakdown to the kick and the roots, then back in',
		],
	},
	{
		id: 'secret',
		trim: 0.95,
		title: 'Gumdrop Music Box',
		...fromGame('secret', 'Gumdrop Music Box'),
		contrast: 7,
		lift: 0,
		arp: false,
		devices: ['music box moves to the dominant for the middle', 'octave doubling for the return'],
	},
	{
		id: 'citadel',
		trim: 0.75,
		title: "Mildred's March",
		...fromGame('citadel', "Mildred's March"),
		contrast: 5,
		lift: 2,
		arp: true,
		devices: [
			'march with a contrast section in the subdominant',
			'pad arpeggiated for texture',
			'final key lift',
		],
	},
	{
		id: 'funk',
		trim: 0.79,
		title: 'Funky Fizz',
		origin: { kind: 'original' },
		track: FUNK,
		style:
			'An original funk number: E dorian, a one-bar bass riff, sixteenth-note hats with ghost snares, and off-beat chord stabs.',
		contrast: 5,
		lift: 0,
		arp: false,
		devices: [
			'sixteenth-note groove with ghost notes',
			'riff-and-answer horn line',
			'the fourth section moves up to the IV chord',
			'bass-and-drums breakdown into a shout',
		],
	},
	{
		id: 'groove',
		trim: 0.82,
		title: 'Cocoa Groove',
		origin: { kind: 'original' },
		track: GROOVE,
		style:
			'An original shuffle in twelve steps to the bar: F minor, a boom-bap kick and snare, electric-piano ninths and a sparse bell.',
		contrast: 5,
		lift: 0,
		arp: false,
		devices: [
			'triplet shuffle feel',
			'ninth and seventh chord voicings',
			'sparse bell over the chords',
			'half-bar breakdown into a snare-roll build',
		],
	},
	{
		id: 'tiptoe',
		trim: 1,
		fill: 0.45,
		title: 'Tiptoe Fizz',
		track: TIPTOE,
		origin: {
			kind: 'inspired',
			game: 'Keen 5',
			title: "Just Snoopin' Around",
			chunk: 195,
			facts: 'A A′ B B′ C · 82 s · 120 bpm · C minor',
		},
		style:
			'An original creeper in A minor, built on the ideas of a sneaking cue: a sparse bass that creeps by semitones and approaches each root from below, a clock-like tick on every sixteenth, held open fifths, and a few staccato stabs.',
		contrast: 5,
		lift: 0,
		arp: false,
		devices: [
			'a bass that creeps by semitones and leans in from a leading tone',
			'a constant tick with an off-beat woodblock answering it',
			'layers arrive section by section: bass and tick, then fifths, then stabs and lead',
			'the last section thins back out to the creep',
		],
		build: [
			{
				name: 'A',
				cycles: 8,
				roles: ['bass', 'hats', 'perc'],
				what: 'the bass, the tick and the woodblock only: the sneak begins',
			},
			{
				name: 'A′',
				cycles: 6,
				roles: ['bass', 'hats', 'perc', 'pad'],
				what: 'held open fifths join under the creep',
			},
			{ name: 'B', cycles: 8, roles: 'all', what: 'the stabs and the tiptoeing lead enter' },
			{
				name: 'B′',
				cycles: 4,
				roles: 'all',
				shift: 5,
				octave: true,
				what: 'everything a fourth up, the lead doubled an octave',
			},
			{
				name: 'C',
				cycles: 4,
				roles: ['bass', 'pad', 'perc'],
				what: 'back to the creep, thinning out so the loop can start again',
			},
		],
	},
	{
		id: 'squeeze',
		trim: 0.9,
		title: 'Tight Squeeze',
		track: SQUEEZE,
		origin: {
			kind: 'inspired',
			game: 'Keen 5',
			title: 'Spiro Grip Me Tighter / Make It Tighter',
			chunk: 202,
			facts: 'A B B′ · 59 s · 132 bpm · B minor',
		},
		style:
			'An original tight groove in G minor on the ideas of a building cue: a rising chromatic bass riff with low pickups over a pulsing pedal, tight drums, and layers that arrive section by section.',
		contrast: 0,
		lift: 2,
		arp: false,
		devices: [
			'a bass riff that climbs by semitone fragments, then drops to a pickup',
			'a pedal pulsing on the eighths holds the key',
			'layers arrive section by section: riff and drums, then the arpeggio, then the lead',
			'breakdown and build, then the last time a whole step up',
		],
		build: [
			{
				name: 'A',
				cycles: 8,
				roles: ['bass', 'counter', 'kick', 'snare', 'hats'],
				what: 'the riff, the pedal and the drums',
			},
			{
				name: 'B',
				cycles: 4,
				roles: ['bass', 'counter', 'kick', 'snare', 'hats', 'arp'],
				what: 'a plucked arpeggio joins',
			},
			{ name: 'B′', cycles: 8, roles: 'all', what: 'the lead enters: every layer plays' },
			{ name: 'Breakdown', cycles: 3, roles: [], kind: 'breakdown', what: 'the band drops out' },
			{
				name: 'Build',
				cycles: 1,
				roles: [],
				kind: 'build',
				what: 'a snare roll into the last time',
			},
			{
				name: 'A′',
				cycles: 8,
				roles: 'all',
				shift: 2,
				octave: true,
				what: 'the last time, a whole step up, the lead doubled',
			},
		],
	},
	{
		id: 'map',
		trim: 0.9,
		intro: 0,
		noFill: true,
		form: 'A B A′',
		title: 'Overworld Stroll',
		...fromGame('map', 'Overworld Stroll'),
		contrast: 7,
		lift: 0,
		arp: false,
		devices: [
			'oom-pah walk, with the trio in the dominant',
			'octave doubling on the return, back in the home key',
			'no intro and no fill: the tune leads, and the last section runs straight back into the first',
		],
	},
	{
		id: 'boss',
		trim: 0.8,
		intro: 0,
		noFill: true,
		title: 'Colossus Showdown',
		...fromGame('boss', 'Colossus Showdown'),
		contrast: 5,
		lift: 0,
		arp: true,
		devices: [
			'frantic tempo kept up all the way through',
			'contrast section in the subdominant',
			'octave doubling on the last time, back in the home key so the loop wraps cleanly',
		],
	},
	{
		id: 'aire',
		trim: 0.9,
		fill: 0,
		title: 'Fizz Aire',
		track: AIRE,
		origin: {
			kind: 'inspired',
			game: 'Keen 5',
			title: 'A Shikadi Aire',
			chunk: 200,
			facts: 'A B · 49 s · 120 bpm · C minor · six channels, no drums',
		},
		style:
			'An original stately dance in D minor on the ideas of a courtly cue: long bass notes moving between tonic and dominant, short chord pulses on every beat like a harpsichord, a stepwise tune, and a second line below it. No drums at all.',
		contrast: 7,
		lift: 0,
		arp: false,
		devices: [
			'long bass notes and a leading tone before each cadence',
			'chord pulses on the beat, plucked rather than held',
			'a second line moving in thirds and sixths below the tune',
			'layers arrive one at a time; the middle section goes to the dominant, and the coda thins back out',
		],
		build: [
			{
				name: 'A',
				cycles: 4,
				roles: ['bass', 'bell'],
				what: 'the bass and the chord pulses alone',
			},
			{ name: 'A′', cycles: 6, roles: ['bass', 'bell', 'lead'], what: 'the tune enters over them' },
			{
				name: 'B',
				cycles: 8,
				roles: 'all',
				shift: 7,
				what: 'everything in the dominant, with the second line',
			},
			{
				name: 'A″',
				cycles: 6,
				roles: 'all',
				octave: true,
				what: 'back in D minor, the tune doubled an octave up',
			},
			{
				name: 'C',
				cycles: 4,
				roles: ['bass', 'bell'],
				what: 'thinning back to the bass and the pulses, so the loop can begin again',
			},
		],
	},
];

export type Plan = {
	def: SongDef;
	sections: Section[];
	cycles: number;
	/** Seconds in one run of the song. */
	seconds: number;
	bpm: number;
	/** Forms tried and the one chosen, for the page. */
	form: string;
};

const TARGET_SECONDS = 60;

/** Chooses the form whose length lands nearest a minute, and lays out its sections. */
export function planSong(def: SongDef): Plan {
	const { track } = def;
	if (def.build) {
		const sections = [
			...def.build.map((step) =>
				step.kind ? breakdown(track, step.cycles, step.kind === 'build') : layered(track, step),
			),
			...(def.noFill ? [] : [turnaround(track, def.fill)]),
		];
		const cycles = sections.reduce((total, section) => total + section.cycles, 0);
		return {
			def,
			sections,
			cycles,
			seconds: (cycles * 240) / track.bpm,
			bpm: track.bpm,
			form: def.build
				.filter((step) => !step.kind)
				.map((step) => step.name)
				.join(' '),
		};
	}
	const loop = readTrack(track).cycles;
	const cycleSeconds = 240 / track.bpm;

	/** The forms available when a main section lasts `unit` bars (a whole number of the track's loop). */
	const opening = def.intro === 0 ? [] : [intro(track, def.intro ?? 2)];
	const closing = def.noFill ? [] : [turnaround(track)];

	const formsFor = (unit: number) => {
		const half = Math.max(2, unit / 2);
		const A = (name: string, what: string, options: MainOptions = {}) =>
			main(track, unit, name, what, options);
		const tune = 'the tune as the game plays it';
		const contrast = (name = 'B') =>
			A(
				name,
				`a contrast section ${def.contrast ? `up ${def.contrast} semitones` : 'with a new texture'}`,
				{
					shift: def.contrast,
					arp: def.arp,
				},
			);
		const last = (name = 'A′') =>
			A(
				name,
				`the last tune, doubled an octave up${def.lift ? ` and lifted ${def.lift} semitones` : ''}`,
				{
					shift: def.lift,
					arp: def.arp,
					octave: true,
				},
			);
		const back = A('A', 'back to the tune, now doubled an octave up', { octave: true });
		const drop = [breakdown(track, half - 1, false), breakdown(track, 1, true)];
		return [
			{
				name: 'A B′',
				sections: [
					...opening,
					A('A', tune),
					A(
						'B′',
						`the contrast section doubled an octave up${def.contrast ? `, up ${def.contrast} semitones` : ''}`,
						{
							shift: def.contrast,
							arp: def.arp,
							octave: true,
						},
					),
					...closing,
				],
			},
			{
				name: 'A B A′',
				sections: [...opening, A('A', tune), contrast(), last(), ...closing],
			},
			{
				name: 'A B breakdown A′',
				sections: [...opening, A('A', tune), contrast(), ...drop, last(), ...closing],
			},
			{
				name: 'A B A breakdown A′',
				sections: [...opening, A('A', tune), contrast(), back, ...drop, last(), ...closing],
			},
			{
				name: 'A A B A breakdown A′',
				sections: [
					...opening,
					A('A', tune),
					A('A', 'the tune again with the pad arpeggiated', { arp: def.arp }),
					contrast(),
					back,
					...drop,
					last(),
					...closing,
				],
			},
		];
	};

	// A short loop can play each section twice through; the length decides which suits.
	const units = loop < 8 ? [loop, loop * 2] : [loop];
	const measured = units
		.flatMap((unit) => formsFor(unit))
		.map((form) => ({
			form,
			cycles: form.sections.reduce((total, section) => total + section.cycles, 0),
		}));
	const pinned = def.form ? measured.filter((item) => item.form.name === def.form) : [];
	const pool = pinned.length ? pinned : measured;
	const best = pool.reduce((a, b) =>
		Math.abs(a.cycles * cycleSeconds - TARGET_SECONDS) <=
		Math.abs(b.cycles * cycleSeconds - TARGET_SECONDS)
			? a
			: b,
	);
	return {
		def,
		sections: best.form.sections,
		cycles: best.cycles,
		seconds: best.cycles * cycleSeconds,
		bpm: track.bpm,
		form: best.form.name,
	};
}

// ---------- Playing ----------

export type Song = { plan: Plan; streams: Voice[] };

/**
 * Builds the song as one stream per slot, each arranged across the sections it plays in.
 * With `depth` the voices carry the OPL2-inspired shaping (filter envelope and a partner layer).
 */
export function buildSong(plan: Plan, options: { depth: boolean; placed: boolean }): Song {
	const trim = plan.def.trim;
	const keys = [
		...new Set(plan.sections.flatMap((section) => section.slots.map((slot) => slot.key))),
	];
	const streams: Voice[] = [];
	keys.forEach((key, index) => {
		const voicesIn = (section: Section) => {
			const slot = section.slots.find((candidate) => candidate.key === key);
			if (!slot) return undefined;
			const part = { ...slot.part, g: slot.part.g * trim };
			return options.depth
				? depthVoices(part, index, options.placed)
				: [buildVoice(U, part, 1, true, options.placed)];
		};
		const first = plan.sections.map(voicesIn).find((voices) => voices !== undefined);
		if (!first) return;
		for (let layer = 0; layer < first.length; layer++) {
			streams.push(
				U.arrange(
					...plan.sections.map((section): [number, Voice] => [
						section.cycles,
						voicesIn(section)?.[layer] ?? (U.silence as Voice),
					]),
				),
			);
		}
	});
	return { plan, streams };
}

/**
 * Plays the song in a loop, starting `startCycle` bars in. The arrangement repeats after its last
 * bar, so shifting it by any number of bars still wraps seamlessly.
 */
export function loopSong(song: Song, ctx: AudioContext, startCycle = 0) {
	const whole = U.stack(...song.streams);
	return (startCycle ? shifted(whole, startCycle) : whole).loop({ ctx, bpm: song.plan.bpm });
}

export const shifted = (pattern: Voice, cycle: number): Voice =>
	new U.Pattern((span) =>
		pattern
			.query({
				begin: span.begin.add(U.Fraction.from(cycle)),
				end: span.end.add(U.Fraction.from(cycle)),
			})
			.map((hap) => ({
				whole: hap.whole && {
					begin: hap.whole.begin.sub(U.Fraction.from(cycle)),
					end: hap.whole.end.sub(U.Fraction.from(cycle)),
				},
				part: {
					begin: hap.part.begin.sub(U.Fraction.from(cycle)),
					end: hap.part.end.sub(U.Fraction.from(cycle)),
				},
				value: hap.value,
			})),
	);

/** Schedules every cycle of the song on a context, one `play` a cycle. Used to render offline. */
export function scheduleSong(song: Song, ctx: BaseAudioContext, start = 0.05) {
	const cycleSeconds = 240 / song.plan.bpm;
	const whole = U.stack(...song.streams);
	for (let cycle = 0; cycle < song.plan.cycles; cycle++) {
		shifted(whole, cycle).play({
			ctx: ctx as AudioContext,
			bpm: song.plan.bpm,
			when: start + cycle * cycleSeconds,
			gated: true,
		});
	}
}
