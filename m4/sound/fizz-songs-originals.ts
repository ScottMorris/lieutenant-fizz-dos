// Two original Lieutenant Fizz tracks, a funk and a groove, in the game's own voice format
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: MIT

import type { MusicPart, MusicTrack } from './fizz-game/types.js';

/** Counts the top-level steps of a mini-notation bar; `[a,b,c]` is one step. */
function stepCount(bar: string): number {
	let depth = 0;
	let count = 0;
	let inToken = false;
	for (const letter of bar.trim()) {
		if (letter === '[') depth++;
		if (letter === ']') depth--;
		const space = /\s/.test(letter) && depth === 0;
		if (space) inToken = false;
		else if (!inToken) {
			count++;
			inToken = true;
		}
	}
	return count;
}

/** A bar of exactly `steps` steps, or an error that names the bar. */
function bar(steps: number, text: string): string {
	const found = stepCount(text);
	if (found !== steps) throw new Error(`Bar has ${found} steps, not ${steps}: ${text}`);
	return `[${text.trim()}]`;
}

/** A cycle-by-cycle pattern: one bar per cycle. */
const bars = (steps: number, list: string[]) =>
	`<${list.map((text) => bar(steps, text)).join(' ')}>`;

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
const kick = (pat: string, g = 0.7): MusicPart =>
	P(pat, { role: 'kick', w: 'sine', d: 0.12, s: 0, r: 0.06, g, slide: 0.09 });
const hats = (pat: string, g = 0.1): MusicPart =>
	P(pat, { role: 'hats', noise: true, d: 0.03, s: 0, r: 0.01, g, hpf: 6000 });
const snare = (pat: string, g = 0.28): MusicPart =>
	P(pat, { role: 'snare', noise: true, d: 0.09, s: 0, r: 0.05, g, hpf: 1600 });

// ---------- Funk: E dorian, sixteenth-note groove, 104 bpm ----------
// One bar per cycle: sixteen steps. Em7 for three bars, the fourth a variation, then A7 twice,
// Em7 again and a B7 turnaround.

const stab = (chord: string) =>
	bar(16, `~ ~ ~ [${chord}] ~ ~ [${chord}] ~ ~ ~ [${chord}] ~ ~ [${chord}] ~ ~`);
const Em7 = 'e4,g4,b4,d5';
const A7 = 'a3,c#4,e4,g4';
const B7 = 'b3,d#4,f#4,a4';

export const FUNK: MusicTrack = {
	bpm: 104,
	parts: [
		// A one-bar riff that starts on the "and" of beat 1 and answers itself on the repeat.
		P(
			bars(16, [
				'~ ~ e5 g5 ~ e5 ~ ~ b5 ~ a5 g5 ~ e5 ~ ~',
				'~ ~ e5 g5 ~ e5 ~ ~ d6 ~ b5 ~ a5 ~ g5 ~',
				'~ ~ e5 g5 ~ e5 ~ ~ b5 ~ a5 g5 ~ e5 ~ ~',
				'~ ~ e5 g5 ~ e5 ~ ~ d6 ~ b5 ~ a5 ~ g5 ~',
				'~ ~ c#5 e5 ~ c#5 ~ ~ a5 ~ g5 e5 ~ c#5 ~ ~',
				'~ ~ c#5 e5 ~ c#5 ~ ~ a5 ~ g5 e5 ~ c#5 ~ ~',
				'~ ~ e5 g5 ~ e5 ~ ~ b5 ~ a5 g5 ~ e5 ~ ~',
				'~ ~ b4 d#5 ~ f#5 ~ ~ a5 ~ f#5 ~ d#5 ~ b4 ~',
			]),
			{ role: 'lead', w: 'square', g: 0.07, lpf: 2600, d: 0.09, s: 0.2 },
		),
		// Chord stabs on the off-sixteenths, like a clavinet or a horn section.
		P(
			`<${[stab(Em7), stab(Em7), stab(Em7), stab(Em7), stab(A7), stab(A7), stab(Em7), stab(B7)].join(' ')}>`,
			{ role: 'counter', w: 'square', g: 0.06, lpf: 2200, a: 0.002, d: 0.06, s: 0, r: 0.04 },
		),
		// A warm electric-piano pad, one chord a bar.
		P(
			'<[e3,g3,b3,d4] [e3,g3,b3,d4] [e3,g3,b3,d4] [e3,g3,b3,d4] [a3,c#4,e4,g4] [a3,c#4,e4,g4] [e3,g3,b3,d4] [b3,d#4,f#4,a4]>',
			{ role: 'pad', w: 'sine', a: 0.02, d: 0.4, s: 0.5, r: 0.3, g: 0.05, lpf: 1800, room: 0.2 },
		),
		// The one-bar bass riff, with the fourth bar of each four answering the first.
		P(
			bars(16, [
				'e2 ~ ~ e2 ~ e3 ~ ~ g2 ~ e2 ~ ~ d3 e3 ~',
				'e2 ~ ~ e2 ~ e3 ~ g2 ~ a2 ~ b2 ~ d3 ~ e3',
				'e2 ~ ~ e2 ~ e3 ~ ~ g2 ~ e2 ~ ~ d3 e3 ~',
				'e2 ~ ~ e2 ~ e3 ~ g2 ~ a2 ~ b2 ~ d3 ~ e3',
				'a2 ~ ~ a2 ~ a3 ~ ~ c#3 ~ a2 ~ ~ g2 a2 ~',
				'a2 ~ ~ a2 ~ a3 ~ ~ c#3 ~ a2 ~ ~ g2 a2 ~',
				'e2 ~ ~ e2 ~ e3 ~ ~ g2 ~ e2 ~ ~ d3 e3 ~',
				'b2 ~ ~ b2 ~ b3 ~ ~ a2 ~ g2 ~ e2 ~ d3 ~',
			]),
			{ role: 'bass', w: 'sawtooth', g: 0.26, s: 0.35, d: 0.1, lpf: 900 },
		),
		kick(bar(16, 'c1 ~ ~ ~ ~ ~ c1 ~ ~ ~ c1 ~ ~ ~ ~ ~'), 0.6),
		snare(bar(16, '~ ~ ~ ~ white ~ ~ ~ ~ ~ ~ ~ white ~ ~ ~'), 0.26),
		// Ghost notes between the backbeats: the sound of a funk drummer's left hand.
		P(bar(16, '~ ~ white ~ ~ white ~ ~ ~ white ~ white ~ ~ white ~'), {
			role: 'perc',
			noise: true,
			d: 0.03,
			s: 0,
			r: 0.02,
			g: 0.05,
			hpf: 2600,
		}),
		// Sixteenth hats the whole way through: the groove's constant pulse.
		hats('white*16', 0.05),
	],
};

// ---------- Groove: F minor, a swung shuffle in twelve steps, 88 bpm ----------
// One bar per cycle: twelve steps, four beats of triplets. Fm9, Dbmaj7, Ab6, C7, then again with a
// Bbm7 in the second pass.

export const GROOVE: MusicTrack = {
	bpm: 88,
	parts: [
		P(
			bars(12, [
				'~ ~ c5 ~ d#5 f5 ~ ~ g5 ~ f5 d#5',
				'~ ~ c5 ~ c#5 d#5 ~ ~ f5 ~ d#5 c#5',
				'~ ~ c5 ~ d#5 g#5 ~ ~ g5 ~ f5 d#5',
				'~ ~ e5 ~ g5 a#5 ~ ~ g#5 ~ g5 e5',
				'~ ~ c5 ~ d#5 f5 ~ ~ g5 ~ f5 d#5',
				'~ ~ c5 ~ c#5 d#5 ~ ~ f5 ~ d#5 c#5',
				'~ ~ c#5 ~ f5 g#5 ~ ~ a#5 ~ g#5 f5',
				'~ ~ e5 ~ g5 a#5 ~ ~ g#5 ~ g5 e5',
			]),
			{
				role: 'lead',
				w: 'sine',
				a: 0.012,
				d: 0.2,
				s: 0.35,
				r: 0.25,
				g: 0.1,
				delay: 0.35,
				lpf: 2400,
			},
		),
		// Electric-piano chords, a ninth and seventh voicing for each bar.
		P(
			'<[f3,g#3,c4,d#4,g4] [c#3,f3,g#3,c4] [g#3,c4,d#4,f4] [c3,e3,g3,a#3] [f3,g#3,c4,d#4,g4] [c#3,f3,g#3,c4] [a#2,c#3,f3,g#3] [c3,e3,g3,a#3]>',
			{ role: 'pad', w: 'sine', a: 0.03, d: 0.5, s: 0.6, r: 0.5, g: 0.06, lpf: 1600, room: 0.35 },
		),
		// A sparse high bell on the third triplet of each bar, one note moving through the chords.
		P(
			bars(12, [
				'~ ~ ~ ~ ~ ~ ~ ~ g6 ~ ~ ~',
				'~ ~ ~ ~ ~ ~ ~ ~ f6 ~ ~ ~',
				'~ ~ ~ ~ ~ ~ ~ ~ d#6 ~ ~ ~',
				'~ ~ ~ ~ ~ ~ ~ ~ e6 ~ ~ ~',
				'~ ~ ~ ~ ~ ~ ~ ~ g6 ~ ~ ~',
				'~ ~ ~ ~ ~ ~ ~ ~ f6 ~ ~ ~',
				'~ ~ ~ ~ ~ ~ ~ ~ f6 ~ ~ ~',
				'~ ~ ~ ~ ~ ~ ~ ~ g6 ~ ~ ~',
			]),
			{ role: 'bell', w: 'sine', a: 0.002, d: 0.4, s: 0.05, r: 0.5, g: 0.06, delay: 0.4 },
		),
		P(
			bars(12, [
				'f1 ~ ~ f1 ~ ~ ~ c2 ~ f1 ~ g#1',
				'c#2 ~ ~ c#2 ~ ~ ~ g#1 ~ c#2 ~ f2',
				'g#1 ~ ~ g#1 ~ ~ ~ d#2 ~ g#1 ~ c2',
				'c2 ~ ~ c2 ~ ~ ~ g1 ~ a#1 ~ b1',
				'f1 ~ ~ f1 ~ ~ ~ c2 ~ f1 ~ g#1',
				'c#2 ~ ~ c#2 ~ ~ ~ g#1 ~ c#2 ~ f2',
				'a#1 ~ ~ a#1 ~ ~ ~ f2 ~ a#1 ~ c2',
				'c2 ~ ~ c2 ~ ~ ~ g1 ~ a#1 ~ b1',
			]),
			{ role: 'bass', g: 0.3, s: 0.4, d: 0.15, lpf: 700 },
		),
		kick(bar(12, 'c1 ~ ~ ~ ~ ~ ~ ~ c1 ~ c1 ~'), 0.6),
		snare(bar(12, '~ ~ ~ white ~ ~ ~ ~ ~ white ~ ~'), 0.24),
		// Hats on the first and third triplet of each beat: the shuffle.
		hats(bar(12, 'white ~ white white ~ white white ~ white white ~ white'), 0.05),
	],
};

// ---------- Tiptoe: A minor, creeping bass and ticking percussion, 118 bpm ----------
// Inspired by the idea of a sneaking cue: a sparse bass that creeps by semitones and approaches
// each root from below, a constant tick on the sixteenths, held open fifths and a few stabs.
// One bar per cycle: sixteen steps.

export const TIPTOE: MusicTrack = {
	bpm: 118,
	parts: [
		// The creeping bass: two or three notes a bar, with a leading tone (g#) before each A.
		P(
			bars(16, [
				'a1 ~ ~ ~ ~ ~ ~ ~ f1 ~ ~ ~ ~ ~ ~ ~',
				'e1 ~ ~ ~ ~ ~ d#1 ~ e1 ~ ~ ~ ~ ~ g#1 ~',
				'a1 ~ ~ ~ ~ ~ ~ ~ f1 ~ ~ ~ ~ ~ ~ ~',
				'e1 ~ ~ ~ ~ ~ d#1 ~ e1 ~ ~ g#1 ~ ~ a1 ~',
				'd2 ~ ~ ~ ~ ~ ~ ~ c2 ~ ~ ~ ~ ~ b1 ~',
				'a1 ~ ~ ~ ~ ~ ~ ~ f1 ~ ~ ~ ~ ~ ~ ~',
				'e1 ~ ~ ~ ~ ~ d#1 ~ e1 ~ ~ ~ ~ ~ g#1 ~',
				'e1 ~ ~ g#1 ~ ~ b1 ~ e1 ~ ~ ~ ~ ~ ~ ~',
			]),
			{ role: 'bass', g: 0.38, s: 0.3, d: 0.1, r: 0.06, lpf: 800 },
		),
		// The tick: one noise burst on every sixteenth, like a clock.
		hats('white*16', 0.06),
		// A woodblock that answers the tick off the beat.
		P(bar(16, '~ ~ ~ ~ a5 ~ ~ a5 ~ ~ a5 ~ ~ ~ a5 ~'), {
			role: 'perc',
			w: 'square',
			d: 0.03,
			s: 0,
			r: 0.02,
			g: 0.065,
			lpf: 3200,
		}),
		// Held open fifths: the long notes under the creeping.
		P('<[a3,e4] [a3,e4] [f3,c4] [e3,b3] [a3,e4] [a3,e4] [f3,c4] [e3,b3]>', {
			role: 'pad',
			w: 'sine',
			a: 0.15,
			d: 0.4,
			s: 0.7,
			r: 0.6,
			g: 0.07,
			lpf: 1500,
			room: 0.3,
		}),
		// A few staccato stabs, then silence.
		P(
			bars(16, [
				'~ ~ ~ ~ ~ ~ ~ ~ ~ ~ [a4,c5,e5] ~ ~ ~ ~ ~',
				'~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ [g#4,b4,e5] ~',
				'~ ~ ~ ~ ~ ~ ~ ~ ~ ~ [a4,c5,e5] ~ ~ [a4,c5,e5] ~ ~',
				'~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ [g#4,b4,e5] ~',
				'~ ~ ~ ~ [d5,f5,a5] ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~',
				'~ ~ ~ ~ ~ ~ ~ ~ ~ ~ [a4,c5,e5] ~ ~ ~ ~ ~',
				'~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ [g#4,b4,e5] ~',
				'~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ [e4,g#4,b4] ~ ~ ~',
			]),
			{ role: 'counter', w: 'square', a: 0.002, d: 0.05, s: 0, r: 0.04, g: 0.1, lpf: 1900 },
		),
		// The lead tiptoes in on the second half, one note at a time.
		P(
			bars(16, [
				'~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~',
				'~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~',
				'~ ~ ~ ~ e5 ~ ~ ~ c5 ~ ~ ~ b4 ~ a4 ~',
				'~ ~ ~ ~ e5 ~ ~ ~ f5 ~ e5 ~ d#5 ~ e5 ~',
				'~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~',
				'~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~',
				'~ ~ a4 ~ c5 ~ e5 ~ ~ ~ a5 ~ g#5 ~ a5 ~',
				'~ ~ g#4 ~ b4 ~ d5 ~ e5 ~ ~ ~ ~ ~ ~ ~',
			]),
			{ role: 'lead', w: 'triangle', a: 0.004, d: 0.1, s: 0.25, r: 0.08, g: 0.12, lpf: 2400 },
		),
	],
};

// ---------- Squeeze: G minor, a rising bass riff over a pulsing pedal, 128 bpm ----------
// Inspired by a tight, building groove: a chromatic bass riff, a steady pedal, tight drums, and
// layers that arrive section by section. One bar per cycle: sixteen steps.

/** A bar of sixteen arpeggio steps that rises and falls through the chord's tones. */
const arpBar = (tones: string[]) => {
	const path = [...tones, ...tones.slice(1, -1).reverse()];
	return Array.from({ length: 16 }, (_, step) => path[step % path.length]).join(' ');
};

export const SQUEEZE: MusicTrack = {
	bpm: 128,
	parts: [
		// The riff climbs by semitone fragments from the root, then drops to a pickup.
		P(
			bars(16, [
				'g1 ~ ~ ~ ~ ~ a1 ~ a#1 ~ ~ ~ b1 ~ c2 ~',
				'~ ~ f1 ~ ~ ~ ~ ~ ~ ~ ~ ~ f#1 ~ ~ ~',
				'g1 ~ ~ ~ ~ ~ a1 ~ a#1 ~ ~ ~ b1 ~ c2 ~',
				'~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ f1 ~ ~ ~',
				'g1 ~ ~ ~ ~ ~ a1 ~ a#1 ~ ~ ~ b1 ~ c2 ~',
				'~ ~ f1 ~ ~ ~ ~ ~ ~ ~ ~ ~ f#1 ~ ~ ~',
				'g1 ~ ~ ~ ~ ~ a1 ~ a#1 ~ ~ ~ b1 ~ c#2 ~',
				'~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ ~ d2 ~ f#1 ~',
			]),
			{ role: 'bass', w: 'sawtooth', g: 0.26, s: 0.4, d: 0.1, r: 0.06, lpf: 900 },
		),
		// The pedal: a single note pulsing on the eighths, holding the key.
		P(bar(16, 'g2 ~ g2 ~ g2 ~ g2 ~ g2 ~ g2 ~ g2 ~ g2 ~'), {
			role: 'counter',
			w: 'square',
			a: 0.002,
			d: 0.06,
			s: 0.2,
			r: 0.04,
			g: 0.08,
			lpf: 800,
		}),
		kick(bar(16, 'c1 ~ ~ ~ ~ ~ ~ c1 c1 ~ ~ ~ ~ ~ ~ ~'), 0.6),
		snare(bar(16, '~ ~ ~ ~ white ~ ~ ~ ~ ~ ~ ~ white ~ ~ ~'), 0.26),
		hats('white*16', 0.045),
		// A plucked arpeggio over Gm, Gm, Eb, F, then Gm, Gm, Cm, D.
		P(
			`<${[
				['g3', 'a#3', 'd4'],
				['g3', 'a#3', 'd4'],
				['d#3', 'g3', 'a#3'],
				['f3', 'a3', 'c4'],
				['g3', 'a#3', 'd4'],
				['g3', 'a#3', 'd4'],
				['c3', 'd#3', 'g3'],
				['d3', 'f#3', 'a3'],
			]
				.map((tones) => bar(16, arpBar(tones)))
				.join(' ')}>`,
			{ role: 'arp', w: 'triangle', a: 0.003, d: 0.1, s: 0.15, r: 0.08, g: 0.07, lpf: 2200 },
		),
		// A short riff in G minor pentatonic over the top.
		P(
			bars(16, [
				'~ ~ ~ ~ d5 ~ f5 ~ g5 ~ ~ ~ f5 ~ d5 ~',
				'~ ~ ~ ~ c5 ~ d5 ~ f5 ~ ~ ~ ~ ~ ~ ~',
				'~ ~ ~ ~ d5 ~ f5 ~ g5 ~ ~ ~ a#5 ~ g5 ~',
				'~ ~ ~ ~ f5 ~ ~ ~ d5 ~ ~ ~ c5 ~ ~ ~',
				'~ ~ ~ ~ d5 ~ f5 ~ g5 ~ ~ ~ f5 ~ d5 ~',
				'~ ~ ~ ~ c5 ~ d5 ~ f5 ~ ~ ~ ~ ~ ~ ~',
				'~ ~ ~ ~ g5 ~ a#5 ~ c6 ~ ~ ~ a#5 ~ g5 ~',
				'~ ~ ~ ~ f#5 ~ ~ ~ a5 ~ ~ ~ ~ ~ ~ ~',
			]),
			{ role: 'lead', w: 'square', g: 0.07, lpf: 2600, d: 0.09, s: 0.2 },
		),
	],
};

// ---------- Aire: D minor, a stately dance with no drums, 112 bpm ----------
// Inspired by a courtly cue: long bass notes that move between tonic and dominant, short chord
// pulses on each beat like a harpsichord, a stepwise tune with a quick run before the cadence,
// and a second line moving in thirds below it. One bar per cycle, in beats.

const DM = 'd4,f4,a4';
const A = 'a3,c#4,e4';
const GM = 'g3,a#3,d4';
const F = 'f3,a3,c4';
const BB = 'a#3,d4,f4';

/** A bar of four chord pulses, two on each of two chords. */
const pulses = (first: string, second: string) =>
	bar(4, `[${first}] [${first}] [${second}] [${second}]`);

export const AIRE: MusicTrack = {
	bpm: 112,
	parts: [
		// Long bass notes: two to a bar, tonic to dominant, with a leading tone before the cadences.
		P(bars(2, ['d2 a1', 'a1 c#2', 'd2 f2', 'g1 a1', 'd2 a1', 'a#1 g1', 'a1 c#2', 'd2 d2']), {
			role: 'bass',
			w: 'triangle',
			g: 0.28,
			a: 0.01,
			d: 0.3,
			s: 0.8,
			r: 0.15,
			lpf: 700,
		}),
		// Chord pulses on every beat: a plucked, harpsichord-like sound.
		P(
			`<${[
				pulses(DM, A),
				pulses(A, A),
				pulses(DM, F),
				pulses(GM, A),
				pulses(DM, A),
				pulses(BB, GM),
				pulses(A, A),
				pulses(DM, DM),
			].join(' ')}>`,
			{ role: 'bell', w: 'triangle', a: 0.002, d: 0.09, s: 0, r: 0.05, g: 0.1, lpf: 3200 },
		),
		// The tune: stepwise, arching up and settling back, with a quick run in the seventh bar.
		P(
			bars(4, [
				'd5 f5 a5 g5',
				'f5 e5 [f5 g5] e5',
				'd5 f5 a5 [g5 f5]',
				'e5 c#5 e5 ~',
				'd5 a5 g5 f5',
				'f5 [e5 d5] g5 f5',
				'e5 g5 [a5 g5 f5 e5] c#5',
				'd5 ~ ~ ~',
			]),
			{ role: 'lead', w: 'square', g: 0.08, a: 0.01, d: 0.2, s: 0.5, r: 0.15, lpf: 2400 },
		),
		// A second line below the tune, moving in thirds and sixths on the half bars.
		P(bars(2, ['f4 e4', 'a4 g4', 'f4 a4', 'a#4 a4', 'f4 e4', 'a4 a#4', 'g4 a4', 'f4 ~']), {
			role: 'counter',
			w: 'triangle',
			g: 0.08,
			a: 0.02,
			d: 0.2,
			s: 0.6,
			r: 0.2,
			lpf: 2000,
		}),
	],
};
