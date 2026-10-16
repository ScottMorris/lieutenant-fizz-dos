// OPL2 register helpers and an IMF stream writer for the Fizz DOS conversions
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: MIT

/** The IMF clock the Keen games use: register writes are timed in 1/560 s. */
export const IMF_RATE = 560;
/** The effect clock of the Keen AdLib and PC-speaker effects. */
export const EFFECT_RATE = 140;
/** The OPL2's sample clock in Hz. */
const OPL_CLOCK = 49_716;

/** Register offsets of the modulator and carrier operators of each of the nine channels. */
export const OPERATORS: readonly (readonly [number, number])[] = [
	[0x00, 0x03],
	[0x01, 0x04],
	[0x02, 0x05],
	[0x08, 0x0b],
	[0x09, 0x0c],
	[0x0a, 0x0d],
	[0x10, 0x13],
	[0x11, 0x14],
	[0x12, 0x15],
];

export type Operator = {
	/** Frequency multiple, as the 4-bit register value (0 is 0.5, 1 is 1, 2 is 2 ... 10 is 10, 11 is 10, 12 is 12). */
	mult: number;
	/** Total level, 0 (loudest) to 63 in 0.75 dB steps. */
	tl: number;
	/** Key scale level, 0 to 3. */
	ksl?: number;
	/** Attack rate 0 to 15 (15 is instant). */
	ar: number;
	/** Decay rate 0 to 15. */
	dr: number;
	/** Sustain level 0 (loudest) to 15 (silent). */
	sl: number;
	/** Release rate 0 to 15. */
	rr: number;
	/** Hold at the sustain level while the key is down (a held note) instead of decaying through it. */
	sustain?: boolean;
	/** Waveform select: 0 sine, 1 half sine, 2 absolute sine, 3 quarter-sine pulses. */
	wave?: number;
	vibrato?: boolean;
	tremolo?: boolean;
};

export type Patch = {
	name: string;
	mod: Operator;
	car: Operator;
	/** Modulator feedback, 0 to 7. */
	feedback: number;
	/** 0 is FM (the modulator drives the carrier); 1 adds both operators. */
	additive?: boolean;
};

/** A register write at a time, in IMF ticks. */
export type Write = { tick: number; register: number; value: number };

export class Stream {
	readonly writes: Write[] = [];
	at(tick: number, register: number, value: number) {
		this.writes.push({ tick: Math.max(0, Math.round(tick)), register, value: value & 0xff });
	}
}

/** The OPL F-number and block that make `hz`, with the F-number kept as high as it goes for resolution. */
export function fnumber(hz: number): { fnum: number; block: number } {
	const safe = Math.min(Math.max(hz, 16), 6000);
	for (let block = 0; block < 8; block++) {
		const fnum = Math.round((safe * 2 ** (20 - block)) / OPL_CLOCK);
		if (fnum <= 1023) return { fnum, block };
	}
	return { fnum: 1023, block: 7 };
}

const operatorRegisters = (op: Operator) => ({
	r20:
		(op.tremolo ? 0x80 : 0) | (op.vibrato ? 0x40 : 0) | (op.sustain ? 0x20 : 0) | (op.mult & 0x0f),
	r40: ((op.ksl ?? 0) << 6) | (op.tl & 0x3f),
	r60: ((op.ar & 0x0f) << 4) | (op.dr & 0x0f),
	r80: ((op.sl & 0x0f) << 4) | (op.rr & 0x0f),
	re0: (op.wave ?? 0) & 3,
});

/** Writes a patch's registers to a channel, with `extraTl` (steps of 0.75 dB) added to the carrier. */
export function setPatch(s: Stream, tick: number, channel: number, patch: Patch, extraTl = 0) {
	const [mod, car] = OPERATORS[channel];
	const m = operatorRegisters(patch.mod);
	const c = operatorRegisters(patch.car);
	s.at(tick, 0x20 + mod, m.r20);
	s.at(tick, 0x20 + car, c.r20);
	s.at(tick, 0x40 + mod, m.r40);
	s.at(tick, 0x40 + car, (c.r40 & 0xc0) | Math.min(63, (c.r40 & 0x3f) + extraTl));
	s.at(tick, 0x60 + mod, m.r60);
	s.at(tick, 0x60 + car, c.r60);
	s.at(tick, 0x80 + mod, m.r80);
	s.at(tick, 0x80 + car, c.r80);
	s.at(tick, 0xe0 + mod, m.re0);
	s.at(tick, 0xe0 + car, c.re0);
	s.at(tick, 0xc0 + channel, ((patch.feedback & 7) << 1) | (patch.additive ? 1 : 0));
}

/** Sets only the carrier level of a channel (the note's volume). */
export function setLevel(s: Stream, tick: number, channel: number, patch: Patch, extraTl: number) {
	const car = OPERATORS[channel][1];
	s.at(tick, 0x40 + car, ((patch.car.ksl ?? 0) << 6) | Math.min(63, patch.car.tl + extraTl));
}

/** Sets the pitch of a channel and its key bit. */
export function setPitch(s: Stream, tick: number, channel: number, hz: number, keyOn: boolean) {
	const { fnum, block } = fnumber(hz);
	s.at(tick, 0xa0 + channel, fnum & 0xff);
	s.at(tick, 0xb0 + channel, (keyOn ? 0x20 : 0) | (block << 2) | (fnum >> 8));
}

/** Releases the key of a channel, leaving the pitch bits as they are. */
export function keyOff(s: Stream, tick: number, channel: number, hz: number) {
	const { fnum, block } = fnumber(hz);
	s.at(tick, 0xb0 + channel, (block << 2) | (fnum >> 8));
}

/** Attenuation in 0.75 dB steps for a gain relative to a reference gain. */
export function attenuation(gain: number, reference: number): number {
	if (gain <= 0) return 63;
	return Math.max(0, Math.min(63, Math.round((20 * Math.log10(reference / gain)) / 0.75)));
}

/**
 * Encodes writes as a Keen-style Type-1 IMF chunk (a 16-bit payload length, then four bytes a write: register, value
 * and the delay in 1/560 s before the next), padded with silence to `totalTicks` so a loop restarts exactly.
 */
export function toImf(s: Stream, totalTicks: number): Uint8Array {
	const writes = s.writes
		.map((w, order) => ({ ...w, order }))
		.sort((a, b) => a.tick - b.tick || a.order - b.order);
	const records: number[] = [];
	const at = (i: number) => Math.min(writes[i].tick, totalTicks);
	for (let i = 0; i < writes.length; i++) {
		const next = i + 1 < writes.length ? at(i + 1) : totalTicks;
		let delay = Math.max(0, next - at(i));
		// A delay of 65535 ticks is 117 s; no cue here comes close.
		delay = Math.min(delay, 0xffff);
		records.push(writes[i].register, writes[i].value, delay & 0xff, delay >> 8);
	}
	const out = new Uint8Array(2 + records.length);
	out[0] = records.length & 0xff;
	out[1] = records.length >> 8;
	out.set(records, 2);
	return out;
}
