// The opening-titles score (Liminal HQ presents, Lieutenant Fizz, the chase), laid out as a song for the lab
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: MIT

import type { MusicPart, MusicTrack } from './fizz-game/types.js';
import { arpeggiate, transposeNotes, type Section, type SongDef } from './fizz-songs.js';

/**
 * One bar is 1.6 s at 150 bpm, and the sequence's sections land on bars:
 *
 *   bars 1-2  Presents  0.0-3.2 s   the Liminal HQ mark slides in (a whoosh and a falling bell sparkle), "presents"
 *   bars 3-4  Words     3.2-6.4 s   Lieutenant whips in on beat 1, Fizz on the and of 2, the slam on beat 4; a roll builds
 *   bars 5-8  Chase     6.4-12.8 s  the band drops in on the downbeat with the title theme, Ben running from the Zargs
 *
 * The harmony runs Cmaj7 Fmaj7 | Ab Bb | C Am F G: the flat-six and flat-seven under the words lift into the title's
 * own C major, and bar 8's G turns straight back into bar 1 as the picture fades to black.
 */
const BPM = 150;

const P = (notes: string, o: Partial<MusicPart> = {}): MusicPart => ({
	notes,
	w: 'triangle',
	a: 0.005,
	d: 0.12,
	s: 0.3,
	r: 0.08,
	g: 0.2,
	...o,
});

// ---------- The bars, one string per part, eight bars each ----------

const PAD_CHORDS =
	'<[c4,e4,g4,b4] [f3,a3,c4,e4] [g#3,c4,d#4] [a#3,d4,f4] [c4,e4,g4] [a3,c4,e4] [f3,a3,c4] [g3,b3,d4]>';

/** The lead: silent under the logo, three hits for the words and the pickup, then the title theme's first four bars. */
const LEAD =
	'<~ ~ [g#4 ~ ~ c5 ~ ~ d#5 ~] [~ ~ ~ ~ a#4 d5 f5 g5] [c5 ~ g4 c5 e5 g5 e5 c5] [a4 ~ e4 a4 c5 e5 c5 a4] [f4 a4 c5 f5 e5 c5 a4 c5] [g4 b4 d5 g5 f5 d5 b4 g4]>';

/** The sparkle as the mark lands (falling through Cmaj7), and an answer as "presents" comes up. */
const BELL = '<[e6 ~ b5 ~ g5 ~ e5 c6] [~ ~ a5 ~ e6 ~ ~ ~] ~ ~ ~ ~ ~ ~>';

/** Roots held under the logo, pumping octaves under the words and the chase. */
const BASS =
	'<c2 f1 [g#1 g#2]*4 [a#1 a#2 a#1 a#2 a#1 a#2 c3 d3] [c2 c3]*4 [a1 a2]*4 [f1 f2]*4 [g1 g2 g1 g2 g1 g2 b1 d2]>';

/** Sixteenths climbing the chord under the words; the chase keeps them going an octave down from the lead. */
const ARP =
	'<~ ~ [g#3 c4 d#4 c4]*4 [a#3 d4 f4 d4]*4 [c4 e4 g4 e4]*4 [a3 c4 e4 c4]*4 [f3 a3 c4 a3]*4 [g3 b3 d4 b3]*4>';

const KICK =
	'<~ ~ [c1 ~ ~ c1 ~ ~ c1 ~] [c1 ~ c1 ~ c1 c1 c1 c1] [c1 ~ ~ c1 c1 ~ ~ ~] [c1 ~ ~ c1 c1 ~ ~ ~] [c1 ~ ~ c1 c1 ~ ~ ~] [c1 ~ ~ c1 c1 ~ c1 ~]>';

/** The slam on beat 4 of bar 3, a roll that doubles up through bar 4, then the backbeat. */
const SNARE =
	'<~ ~ [~ ~ ~ ~ ~ ~ white ~] [~ white ~ white [white white] [white white] [white white white white] [white white white white]] [~ white ~ white] [~ white ~ white] [~ white ~ white] [~ white ~ white]>';

const HATS = '<~ [~ white]*4 white*8 white*8 white*8 white*8 white*8 white*8>';

/** Whooshes: the mark sliding in, Lieutenant and Fizz whipping across, Ben bursting on. */
const WHOOSH = '<[white ~ ~ ~ ~ ~ ~ ~] ~ [white ~ ~ white ~ ~ ~ ~] ~ [white ~ ~ ~ ~ ~ ~ ~] ~ ~ ~>';

/** A crash on the slam and on the chase's downbeat. */
const CRASH = '<~ ~ [~ ~ ~ ~ ~ ~ white ~] ~ white ~ ~ ~>';

// ---------- Classic ----------

/**
 * The Classic loop: the eight bars as one track, as the game's other cues are written. A square lead as the title
 * theme has, a quiet sine pad, a triangle bass, a soft square arpeggio and the kit; the levels sit with the title.
 */
export const OPENING_TITLES_LOOP: MusicTrack = {
	bpm: BPM,
	parts: [
		P(LEAD, { role: 'lead', w: 'square', g: 0.075, lpf: 2600, d: 0.1 }),
		P(PAD_CHORDS, {
			role: 'pad',
			w: 'sine',
			a: 0.05,
			d: 0.5,
			s: 0.7,
			r: 1.1,
			g: 0.05,
			lpf: 1400,
			room: 0.3,
		}),
		P(BELL, {
			role: 'bell',
			w: 'sine',
			a: 0.002,
			d: 0.35,
			s: 0.05,
			r: 0.4,
			g: 0.08,
			delay: 0.4,
		}),
		P(BASS, { role: 'bass', g: 0.3, s: 0.45 }),
		P(ARP, { role: 'arp', w: 'square', g: 0.035, lpf: 1800, d: 0.06 }),
		P(KICK, { role: 'kick', w: 'sine', d: 0.12, s: 0, r: 0.06, g: 0.6, slide: 0.09 }),
		P(SNARE, { role: 'snare', noise: true, d: 0.09, s: 0, r: 0.05, g: 0.24, hpf: 1600 }),
		P(HATS, { role: 'hats', noise: true, d: 0.03, s: 0, r: 0.01, g: 0.08, hpf: 6000 }),
		P(WHOOSH, { role: 'perc', noise: true, a: 0.06, d: 0.28, s: 0, r: 0.1, g: 0.12, hpf: 2500 }),
		P(CRASH, { role: 'perc', noise: true, a: 0.001, d: 0.7, s: 0, r: 0.3, g: 0.12, hpf: 4500 }),
	],
};

// ---------- Enhanced ----------

/** Picks one section's bars out of an eight-bar `<...>` string: `bars(LEAD, 4, 8)` is the chase's four. */
function bars(notes: string, from: number, to: number): string {
	const inner = notes.trim().replace(/^<|>$/g, '');
	const items: string[] = [];
	let depth = 0;
	let item = '';
	for (const ch of inner) {
		if (ch === '[') depth++;
		if (ch === ']') depth--;
		if (ch === ' ' && depth === 0) {
			if (item) items.push(item);
			item = '';
		} else item += ch;
	}
	if (item) items.push(item);
	return `<${items.slice(from, to).join(' ')}>`;
}

type Voices = Record<string, MusicPart>;

const [lead, pad, bell, bass, arp, kick, snare, hats, whoosh, crash] =
	OPENING_TITLES_LOOP.parts as [
		MusicPart,
		MusicPart,
		MusicPart,
		MusicPart,
		MusicPart,
		MusicPart,
		MusicPart,
		MusicPart,
		MusicPart,
		MusicPart,
	];

const cut = (part: MusicPart, from: number, to: number, o: Partial<MusicPart> = {}): MusicPart => ({
	...part,
	notes: bars(part.notes, from, to),
	...o,
});

/**
 * Presents: the room opens up. The pad is wider and wetter, the bell sparkle is doubled an octave up and echoes,
 * a pink-noise swell rises through bar 2 into the words, and the bass only stands on its roots.
 */
const PRESENTS: Voices = {
	pad: cut(pad, 0, 2, { g: 0.07, a: 0.4, room: 0.6 }),
	bell: cut(bell, 0, 2, { g: 0.09, delay: 0.45, room: 0.5 }),
	bellHigh: cut(bell, 0, 2, {
		notes: transposeNotes(bars(BELL, 0, 2), 12),
		g: 0.035,
		delay: 0.5,
		room: 0.6,
	}),
	bass: cut(bass, 0, 2, { s: 0.8, r: 0.6, g: 0.22 }),
	hats: cut(hats, 0, 2, { g: 0.05 }),
	whoosh: cut(whoosh, 0, 2),
	swell: P('<~ pink>', {
		role: 'perc',
		noise: true,
		a: 1.4,
		d: 0.2,
		s: 0.8,
		r: 0.05,
		g: 0.08,
		lpf: 3000,
	}),
};

/**
 * Words: the hits. The lead's three notes are doubled by a sawtooth brass an octave down, so Lieutenant, Fizz and
 * the slam land hard; the arpeggio climbs under them, and the roll doubles up through bar 4 as Classic's does.
 */
const WORDS: Voices = {
	lead: cut(lead, 2, 4),
	brass: cut(lead, 2, 4, {
		notes: transposeNotes(bars(LEAD, 2, 4), -12),
		role: 'counter',
		w: 'sawtooth',
		g: 0.05,
		lpf: 1600,
		d: 0.18,
		s: 0.2,
	}),
	pad: cut(pad, 2, 4, { g: 0.06 }),
	bass: cut(bass, 2, 4),
	arp: cut(arp, 2, 4),
	kick: cut(kick, 2, 4),
	snare: cut(snare, 2, 4),
	hats: cut(hats, 2, 4),
	whoosh: cut(whoosh, 2, 4),
	crash: cut(crash, 2, 4),
};

/**
 * Chase: the full band on the title theme, with the theme doubled an octave up as the engine's last tune is, the
 * pad spread into an eighth-note arpeggio, and the backbeat. Bar 8 ends on G so the loop turns back to bar 1.
 */
const CHASE: Voices = {
	lead: cut(lead, 4, 8),
	octave: cut(lead, 4, 8, { notes: transposeNotes(bars(LEAD, 4, 8), 12), g: lead.g * 0.55 }),
	pad: cut(pad, 4, 8),
	padArp: {
		...pad,
		notes: transposeNotes(arpeggiate(bars(PAD_CHORDS, 4, 8)), 12),
		role: 'arp',
		w: 'triangle',
		a: 0.005,
		d: 0.1,
		s: 0.2,
		r: 0.1,
		g: 0.05,
		lpf: 2400,
		room: 0.3,
	},
	bass: cut(bass, 4, 8),
	arp: cut(arp, 4, 8, { g: 0.03 }),
	kick: cut(kick, 4, 8),
	snare: cut(snare, 4, 8),
	hats: cut(hats, 4, 8),
	whoosh: cut(whoosh, 4, 8),
	crash: cut(crash, 4, 8),
};

const section = (name: string, cycles: number, what: string, voices: Voices): Section => ({
	name,
	cycles,
	what,
	slots: Object.entries(voices).map(([key, part]) => ({ key, part })),
});

/** The three sections of the sequence, in the song format the lab plays, renders and converts. */
export const OPENING_TITLES_SECTIONS: Section[] = [
	section(
		'Presents',
		2,
		'the Liminal HQ mark slides in on a whoosh and a falling bell sparkle, with a swell into the words',
		PRESENTS,
	),
	section(
		'Words',
		2,
		'Lieutenant and Fizz whip in, the slam lands on beat 4 of bar 3 and a snare roll builds',
		WORDS,
	),
	section(
		'Chase',
		4,
		'the band drops in on the title theme, doubled an octave up, with the pad as an arpeggio',
		CHASE,
	),
];

/** The opening titles as a song: eight bars at 150 bpm, looping from the G in bar 8 back into bar 1. */
export const OPENING_TITLES_SONG: SongDef = {
	id: 'opening-titles',
	title: 'Opening Titles',
	style:
		'A pre-title menu sequence: Liminal HQ presents, Lieutenant Fizz, then the chase. Candidate for the opening sequence before the title menu.',
	track: OPENING_TITLES_LOOP,
	contrast: 0,
	lift: 0,
	arp: false,
	trim: 0.8,
	origin: { kind: 'original' },
	intro: 0,
	noFill: true,
	sections: OPENING_TITLES_SECTIONS,
	devices: [
		'three sections that land on bars: the mark (2), the words (2) and the chase (4)',
		'flat-six and flat-seven harmony under the words that lifts into the title theme in C major',
		'the last bar ends on G so the loop turns straight back into the first',
	],
};
