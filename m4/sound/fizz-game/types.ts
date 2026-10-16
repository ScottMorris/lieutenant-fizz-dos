// The audio data types of the Lieutenant Fizz engine, as the lab needs them
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: MIT

// Mirrors packages/engine/src/audio.ts and sound-field.ts in the game repository.

export type OscillatorType = 'sine' | 'square' | 'sawtooth' | 'triangle';

export type PartRole =
	| 'bass'
	| 'kick'
	| 'snare'
	| 'pad'
	| 'drone'
	| 'lead'
	| 'counter'
	| 'bell'
	| 'hats'
	| 'perc'
	| 'arp';

export interface SfxVoice {
	/** Note name (`e4`) or noise type (`white`, `pink`, `brown`). */
	n: string;
	w?: OscillatorType;
	a?: number;
	d?: number;
	r?: number;
	g: number;
	lpf?: number;
	hpf?: number;
	slide?: number;
	nudge?: number;
}

export interface MusicPart {
	notes: string;
	w?: OscillatorType;
	noise?: boolean;
	a?: number;
	d?: number;
	s?: number;
	r?: number;
	g: number;
	lpf?: number;
	hpf?: number;
	slide?: number;
	room?: number;
	delay?: number;
	role?: PartRole;
}

export interface MusicTrack {
	bpm: number;
	parts: MusicPart[];
}

export interface AudioPatterns {
	sfx: Record<string, SfxVoice[]>;
	music: Record<string, MusicTrack>;
	captionSfx: Record<string, string>;
}
