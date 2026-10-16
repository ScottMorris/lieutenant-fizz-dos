// Converts Lieutenant Fizz sound effects into an OPL2 IMF stream and a PC-speaker divisor stream for the DOS build
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: MIT

import { noteToFrequency } from '@liminal-hq/undertone';

import type { SfxVoice } from '../fizz-game/types.js';
import {
	EFFECT_RATE,
	IMF_RATE,
	Stream,
	attenuation,
	keyOff,
	setLevel,
	setPatch,
	setPitch,
	toImf,
	type Patch,
} from './opl.js';
import { PATCHES, patchForEffect } from './patches.js';

/** The OPL channel the effects use; music keeps to channels 0 to 4 and the rhythm-mode drums on 6 to 8. */
export const EFFECT_CHANNEL = 5;
/** The PC speaker's timer clock, so a divisor is 1193180 / frequency. */
export const PIT_CLOCK = 1_193_180;

const NOISE_HZ: Record<string, number> = { white: 3500, pink: 1400, brown: 520 };

type Segment = {
	voice: SfxVoice;
	index: number;
	start: number;
	/** Seconds the voice is heard: its attack and decay, and half the release. */
	length: number;
	hz: number;
	noise: boolean;
};

export type SfxConversion = {
	seconds: number;
	opl: { imf: Uint8Array; ticks: number };
	speaker: { divisors: Uint16Array; bytes: number };
};

const isNoise = (name: string) => name === 'white' || name === 'pink' || name === 'brown';

function segmentsOf(voices: SfxVoice[]): Segment[] {
	return voices.map((voice, index) => ({
		voice,
		index,
		start: voice.nudge ?? 0,
		length: (voice.a ?? 0.001) + (voice.d ?? 0.1) + (voice.r ?? 0.05) * 0.5,
		hz: isNoise(voice.n) ? (NOISE_HZ[voice.n] ?? 2000) : noteToFrequency(voice.n),
		noise: isNoise(voice.n),
	}));
}

/** How loud a voice is `t` seconds into its own life, as a share of its gain. */
function level(seg: Segment, t: number): number {
	const a = seg.voice.a ?? 0.001;
	const d = seg.voice.d ?? 0.1;
	if (t < a) return seg.voice.g * (t / a);
	const decay = Math.max(0, 1 - (t - a) / (d + (seg.voice.r ?? 0.05) * 0.5));
	return seg.voice.g * decay * decay;
}

/** The pitch of a voice: a slide glides down an octave to the note over the slide time, as Undertone's does. */
function pitch(seg: Segment, t: number): number {
	const slide = seg.voice.slide ?? 0;
	return slide > 0 && t < slide ? seg.hz * 2 ** (1 - t / slide) : seg.hz;
}

function seeded(seed: number) {
	let s = seed >>> 0;
	return () => {
		s = (Math.imul(s, 1664525) + 1013904223) >>> 0;
		return s / 4294967296;
	};
}

/** The voice heard at a frame: the one that started last (a jingle's notes follow each other), taking turns on a tie. */
function heard(segments: Segment[], t: number, frame: number): Segment | undefined {
	const live = segments.filter(
		(g) => t >= g.start && t < g.start + g.length && level(g, t - g.start) > 0.002,
	);
	if (!live.length) return undefined;
	const newest = Math.max(...live.map((g) => g.start));
	const tied = live.filter((g) => g.start === newest);
	return tied[frame % tied.length];
}

/** Converts one effect. Both outputs run on the Keen effect clock of 140 frames a second. */
export function convertSfx(voices: SfxVoice[]): SfxConversion {
	const segments = segmentsOf(voices);
	const seconds = Math.max(...segments.map((g) => g.start + g.length));
	const frames = Math.ceil(seconds * EFFECT_RATE) + 1;
	const loudest = Math.max(...voices.map((v) => v.g));
	const ticksPerFrame = IMF_RATE / EFFECT_RATE;

	// ---- OPL2: one channel, a patch for the voice being heard, its level and pitch moved every frame ----
	const s = new Stream();
	s.at(0, 0x01, 0x20);
	const rand = seeded(2026);
	let patchOn: Patch | undefined;
	let sounding: Segment | undefined;
	let lastHz = 440;
	let lastTl = -1;
	for (let frame = 0; frame < frames; frame++) {
		const t = frame / EFFECT_RATE;
		const at = frame * ticksPerFrame;
		const seg = heard(segments, t, frame);
		if (!seg) {
			if (sounding) keyOff(s, at, EFFECT_CHANNEL, lastHz);
			sounding = undefined;
			continue;
		}
		const age = t - seg.start;
		const tl = attenuation(level(seg, age), loudest);
		const hz = seg.noise ? seg.hz * (0.55 + 0.9 * rand()) : pitch(seg, age);
		const patch = seg.noise ? PATCHES.noise : patchForEffect(seg.voice.w);
		const started = sounding?.index !== seg.index && !(seg.noise && sounding?.noise);
		if (started && sounding) keyOff(s, at, EFFECT_CHANNEL, lastHz);
		if (patchOn !== patch) {
			setPatch(s, at + (started && sounding ? 1 : 0), EFFECT_CHANNEL, patch, tl);
			patchOn = patch;
		} else if (tl !== lastTl) {
			setLevel(s, at + 1, EFFECT_CHANNEL, patch, tl);
		}
		setPitch(s, at + 2, EFFECT_CHANNEL, hz, true);
		sounding = seg;
		lastHz = hz;
		lastTl = tl;
	}
	if (sounding) keyOff(s, frames * ticksPerFrame, EFFECT_CHANNEL, lastHz);
	const ticks = frames * ticksPerFrame + 8;

	// ---- PC speaker: one square voice, a divisor a frame; noise is a random pitch, and quiet is thinned out ----
	const divisors = new Uint16Array(frames);
	const noiseRand = seeded(7);
	for (let frame = 0; frame < frames; frame++) {
		const t = frame / EFFECT_RATE;
		const seg = heard(segments, t, frame);
		if (!seg) continue;
		const age = t - seg.start;
		const share = level(seg, age) / loudest;
		if (share < 0.1 || (share < 0.3 && frame % 2 === 1)) continue;
		const hz = seg.noise ? 250 + noiseRand() * (seg.hz * 1.6) : pitch(seg, age);
		divisors[frame] = Math.round(PIT_CLOCK / Math.min(Math.max(hz, 60), 9000));
	}
	let used = divisors.length;
	while (used > 1 && divisors[used - 1] === 0) used--;
	const trimmed = divisors.slice(0, Math.min(divisors.length, used + 1));
	return {
		seconds,
		opl: { imf: toImf(s, ticks), ticks },
		speaker: { divisors: trimmed, bytes: trimmed.length * 2 },
	};
}
