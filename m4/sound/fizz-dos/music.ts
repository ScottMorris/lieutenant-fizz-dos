// Converts a Lieutenant Fizz music track into an OPL2 IMF stream for the DOS build
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: MIT

import type { MusicPart, MusicTrack, PartRole } from '../fizz-game/types.js';
import { readTrack, type SheetEvent } from '../fizz-sheet-events.js';
import {
	IMF_RATE,
	Stream,
	attenuation,
	keyOff,
	setLevel,
	setPatch,
	setPitch,
	toImf,
	type Patch,
	OPERATORS,
} from './opl.js';
import {
	DRUMS,
	DRUM_CHANNEL_HZ,
	DRUM_PATCHES,
	drumForRole,
	patchForPart,
	type DrumName,
} from './patches.js';

/** Melodic channels for music; channel 5 is kept for sound effects and 6 to 8 are the rhythm-mode drums. */
export const MUSIC_CHANNELS = 5;

/** Steps of 0.75 dB taken off every music note, so five channels and the drums together stay clear of the chip's ceiling. */
const MUSIC_TRIM = 6;

/** How much each role matters when there are more notes than channels: the tune and the bass keep their place. */
const PRIORITY: Partial<Record<PartRole, number>> = {
	lead: 9,
	bass: 8,
	counter: 7,
	arp: 6,
	bell: 5,
	pad: 3,
	drone: 2,
};

export type MusicConversion = {
	/** The Type-1 IMF chunk, with its 16-bit length. */
	imf: Uint8Array;
	/** The loop length in IMF ticks (560 a second). */
	ticks: number;
	seconds: number;
	notes: number;
	/** Notes the five melodic channels could not carry, which were left out. */
	dropped: number;
	drumHits: number;
};

type Segment = {
	channel: number;
	part: number;
	start: number;
	end: number;
	hz: number;
	patch: Patch;
	tl: number;
	priority: number;
};

/** Turns a track into register writes: five melodic channels, the rhythm-mode drums, one loop long. */
export function convertMusic(track: MusicTrack): MusicConversion {
	const loop = readTrack(track);
	return convertParts(track.parts, loop.events, track.bpm, loop.cycles);
}

/**
 * Turns timed note events into register writes. `events` carry the index of their part in `parts`; a cycle is four
 * beats, so `cycles` bars at `bpm` is the length of the loop.
 */
export function convertParts(
	parts: MusicPart[],
	events: SheetEvent[],
	bpm: number,
	cycles: number,
): MusicConversion {
	const loop = { events, cycles, seconds: (cycles * 240) / bpm };
	const ticksPerCycle = (240 / bpm) * IMF_RATE;
	const tick = (cycles: number) => Math.round(cycles * ticksPerCycle);
	const total = tick(loop.cycles);
	const s = new Stream();
	s.at(0, 0x01, 0x20);

	const drumFor = parts.map((p) => drumForRole(p.role) ?? (p.noise ? 'cymbal' : undefined));
	const melodic = parts.map((p, i) => ({ p, i })).filter(({ i }) => drumFor[i] === undefined);
	const reference = Math.max(...melodic.map(({ p }) => p.g), 0.01);

	// ---- drums, in rhythm mode ----
	const hits = new Map<DrumName, number[]>();
	for (const e of loop.events) {
		const drum = drumFor[e.part];
		if (drum) hits.set(drum, [...(hits.get(drum) ?? []), tick(e.t0)]);
	}
	if (hits.size) {
		for (const drum of hits.keys()) {
			DRUMS[drum].ops.forEach((offset, i) => {
				const patchOp = DRUM_PATCHES[drum][i];
				s.at(0, 0x20 + offset, (patchOp.sustain ? 0x20 : 0) | patchOp.mult);
				s.at(0, 0x40 + offset, patchOp.tl);
				s.at(0, 0x60 + offset, (patchOp.ar << 4) | patchOp.dr);
				s.at(0, 0x80 + offset, (patchOp.sl << 4) | patchOp.rr);
				s.at(0, 0xe0 + offset, patchOp.wave ?? 0);
			});
		}
		// The bass drum is a two-operator voice on channel 6; its feedback and connection are in C6.
		s.at(0, 0xc6, 0x00);
		DRUM_CHANNEL_HZ.forEach((hz, i) => setPitch(s, 0, 6 + i, hz, false));
		const changes: { tick: number; bit: number; on: boolean }[] = [];
		for (const [drum, ticks] of hits) {
			const sorted = [...new Set(ticks)].sort((a, b) => a - b);
			sorted.forEach((t, i) => {
				const next = sorted[i + 1] ?? total + t;
				const off = Math.min(t + DRUMS[drum].holdTicks, next - 1, total - 1);
				changes.push({ tick: t, bit: DRUMS[drum].bit, on: true });
				changes.push({ tick: Math.max(off, t + 1), bit: DRUMS[drum].bit, on: false });
			});
		}
		changes.sort((a, b) => a.tick - b.tick || Number(a.on) - Number(b.on));
		let mask = 0;
		s.at(0, 0xbd, 0x20);
		for (let i = 0; i < changes.length;) {
			const at = changes[i].tick;
			while (i < changes.length && changes[i].tick === at) {
				mask = changes[i].on ? mask | changes[i].bit : mask & ~changes[i].bit;
				i++;
			}
			s.at(at, 0xbd, 0x20 | mask);
		}
	}

	// ---- melodic notes: give each to a channel, stealing from the least important when there are too many ----
	const notes = loop.events
		.filter((e) => drumFor[e.part] === undefined && e.hz !== undefined)
		.map((e) => ({ ...e, start: tick(e.t0), end: Math.min(tick(e.t1), total - 1) }))
		.sort((a, b) => a.start - b.start || priority(parts[b.part]) - priority(parts[a.part]));
	const segments: Segment[] = [];
	const current: (Segment | undefined)[] = Array(MUSIC_CHANNELS).fill(undefined);
	let dropped = 0;
	for (const e of notes) {
		const part = parts[e.part];
		const pri = priority(part);
		const seg: Omit<Segment, 'channel'> = {
			part: e.part,
			start: e.start,
			end: Math.max(e.end, e.start + 2),
			hz: e.hz!,
			patch: patchForPart(part.role, part.w),
			tl: attenuation(part.g, reference) + MUSIC_TRIM,
			priority: pri,
		};
		const free = current
			.map((c, channel) => ({ c, channel }))
			.filter(({ c }) => !c || c.end <= seg.start);
		let channel: number | undefined = (
			free.find(({ c }) => c?.part === e.part) ??
			free.find(({ c }) => c === undefined) ??
			free[0]
		)?.channel;
		if (channel === undefined) {
			// Everything is sounding: take the channel of the least important note, if this one matters more.
			const busy = current
				.map((c, channel) => ({ c: c!, channel }))
				.sort((a, b) => a.c.priority - b.c.priority || a.c.end - b.c.end);
			if (busy[0].c.priority < pri) {
				channel = busy[0].channel;
				busy[0].c.end = Math.max(busy[0].c.start + 1, seg.start - 1);
			}
		}
		if (channel === undefined) {
			dropped++;
			continue;
		}
		const placed: Segment = { ...seg, channel };
		segments.push(placed);
		current[channel] = placed;
	}
	// Emit each channel's notes in order, with a tick of silence between them so a repeated note strikes again.
	const patchOn: (Patch | undefined)[] = Array(9).fill(undefined);
	const ordered = [...segments].sort((a, b) => a.start - b.start || a.channel - b.channel);
	const byChannel = new Map<number, Segment[]>();
	for (const g of ordered) byChannel.set(g.channel, [...(byChannel.get(g.channel) ?? []), g]);
	for (const list of byChannel.values()) {
		list.forEach((g, i) => {
			const next = list[i + 1];
			if (next) g.end = Math.min(g.end, next.start - 1);
			g.end = Math.max(g.end, g.start + 1);
		});
	}
	for (const g of ordered) {
		if (patchOn[g.channel] !== g.patch) {
			setPatch(s, g.start, g.channel, g.patch, g.tl);
			patchOn[g.channel] = g.patch;
		} else {
			setLevel(s, g.start, g.channel, g.patch, g.tl);
		}
		setPitch(s, g.start, g.channel, g.hz, true);
		keyOff(s, g.end, g.channel, g.hz);
	}
	void OPERATORS;

	const imf = toImf(s, total);
	if (imf.length > 0xffff)
		throw new Error(`The converted cue is too long for an IMF chunk (${imf.length} bytes).`);
	return {
		imf,
		ticks: total,
		seconds: loop.seconds,
		notes: segments.length,
		dropped,
		drumHits: [...hits.values()].reduce((n, h) => n + h.length, 0),
	};
}

function priority(part: MusicPart): number {
	return (part.role && PRIORITY[part.role]) || 4;
}
