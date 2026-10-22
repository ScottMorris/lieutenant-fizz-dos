// Frame effects and timing for the screens: fades and flashes on the frame buffer, tweens in fixed point and a cue timeline.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// The palette is never touched: a frame byte is `level * 16 + colour`, so a fade is a drop of brightness levels in the buffer (`ui::dim`)
// and a flash is one full-brightness colour over the whole frame. Everything here is integers, so the same ticks give the same bytes on
// the native bench and on DOS.

use crate::ui;
use crate::video::{H, W};

/// Darkens the whole frame by `steps` levels. With 0 steps it still turns the clear colour (slot 255) black, as the screen fades always
/// have, so a caller drawing a fade calls it on every frame of the fade, the first included.
pub fn fade(frame: &mut [u8], steps: u8) {
    ui::dim(frame, 0, 0, W as i32, H as i32, steps);
}

/// The whole frame in one full-brightness colour (an EGA index 0..15).
pub fn flash(frame: &mut [u8], colour: u8) {
    frame[..W * H].fill(ui::FULL + colour);
}

/// A linear fade by brightness levels over `len` ticks: out goes from the picture to black, in from black to the picture.
#[derive(Clone, Copy)]
pub struct Ramp {
    pub t: u32,
    pub len: u32,
    pub out: bool,
}

impl Ramp {
    pub const fn new(len: u32, out: bool) -> Ramp {
        Ramp { t: 0, len, out }
    }

    /// The levels to take off the picture now, 0 to 13.
    pub fn steps(&self) -> u8 {
        let s = (self.t * 13 / self.len).min(13) as u8;
        if self.out {
            s
        } else {
            13 - s
        }
    }

    /// Whether the ramp has run its length.
    pub fn done(&self) -> bool {
        self.t >= self.len
    }
}

/// One in Q30 fixed point: eased fractions and tween values are `i64`s scaled by this.
pub const ONE: i64 = 1 << 30;

/// The easings the screens use, as the design's `Easing` curves.
#[derive(Clone, Copy)]
pub enum Ease {
    Linear,
    OutCubic,
    InOutCubic,
    /// Overshoots and settles (`easeOutBack`).
    OutBack,
}

fn mul(a: i64, b: i64) -> i64 {
    (a * b) >> 30
}

/// The eased value for a fraction `u` in `0..=ONE`, in Q30.
pub fn ease(e: Ease, u: i64) -> i64 {
    match e {
        Ease::Linear => u,
        Ease::OutCubic => {
            let v = u - ONE;
            mul(mul(v, v), v) + ONE
        }
        Ease::InOutCubic => {
            if u < ONE / 2 {
                4 * mul(mul(u, u), u)
            } else {
                let w = 2 * u - 2 * ONE;
                mul(mul(u - ONE, w), w) + ONE
            }
        }
        Ease::OutBack => {
            // c1 = 1.70158, c3 = c1 + 1, in Q30.
            const C1: i64 = 1_827_057_613;
            const C3: i64 = C1 + ONE;
            let v = u - ONE;
            let v2 = mul(v, v);
            ONE + mul(C3, mul(v2, v)) + mul(C1, v2)
        }
    }
}

/// A Q30 value rounded half up to an integer, as `Math.round` rounds.
pub fn round_q(v: i64) -> i32 {
    ((v + ONE / 2) >> 30) as i32
}

/// A Q30 value cut toward zero, as `x | 0` cuts.
pub fn trunc_q(v: i64) -> i32 {
    (v / ONE) as i32
}

/// A value moving from `from` to `to` between two times in milliseconds, held at `from` before and at `to` after. The times are whole
/// milliseconds so that the fraction at a tick is exact: `t` ticks at 60 a second are `t * 1000 / 60` ms.
#[derive(Clone, Copy)]
pub struct Tween {
    pub start_ms: u32,
    pub end_ms: u32,
    pub from: i32,
    pub to: i32,
    pub ease: Ease,
}

impl Tween {
    pub const fn new(start_ms: u32, end_ms: u32, from: i32, to: i32, ease: Ease) -> Tween {
        Tween { start_ms, end_ms, from, to, ease }
    }

    /// The value at tick `t`, in Q30, so that sums of tweens round once.
    pub fn q(&self, t: u32) -> i64 {
        let now = i64::from(t) * 1000;
        let (s, e) = (i64::from(self.start_ms) * 60, i64::from(self.end_ms) * 60);
        if now <= s {
            return i64::from(self.from) * ONE;
        }
        if now >= e {
            return i64::from(self.to) * ONE;
        }
        let u = (now - s) * ONE / (e - s);
        i64::from(self.from) * ONE + i64::from(self.to - self.from) * ease(self.ease, u)
    }

    /// The value at tick `t`, rounded half up.
    pub fn at(&self, t: u32) -> i32 {
        round_q(self.q(t))
    }
}

/// An event at a tick.
#[derive(Clone, Copy)]
pub struct Cue<E: Copy> {
    pub at: u32,
    pub ev: E,
}

/// Cues in tick order, handed out once each as the clock passes them. Independent of `story::Reel`, which is keyed by presses.
pub struct Timeline<E: Copy + 'static> {
    cues: &'static [Cue<E>],
    next: usize,
}

impl<E: Copy> Timeline<E> {
    pub const fn new(cues: &'static [Cue<E>]) -> Timeline<E> {
        Timeline { cues, next: 0 }
    }

    pub fn reset(&mut self) {
        self.next = 0;
    }

    /// Calls `f` for every cue at or before tick `t` that has not fired yet.
    pub fn fire(&mut self, t: u32, mut f: impl FnMut(E)) {
        while let Some(c) = self.cues.get(self.next) {
            if c.at > t {
                break;
            }
            f(c.ev);
            self.next += 1;
        }
    }
}

/// Checks the easings' endpoints and the tween's bounds; the bench runs it.
pub fn self_check() -> bool {
    let all = [Ease::Linear, Ease::OutCubic, Ease::InOutCubic, Ease::OutBack];
    let ends = all.iter().all(|&e| ease(e, 0) == 0 && ease(e, ONE) == ONE);
    let tw = Tween::new(200, 1000, -120, 252, Ease::OutBack);
    // 0.2 s is tick 12 and 1.0 s is tick 60.
    ends && tw.at(0) == -120 && tw.at(12) == -120 && tw.at(60) == 252 && tw.at(900) == 252 && round_q(-ONE / 2) == 0 && trunc_q(-ONE / 2) == 0
}
