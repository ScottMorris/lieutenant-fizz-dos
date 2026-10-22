// The opening titles: Liminal HQ presents, the Lieutenant Fizz wordmark and the chase, drawn from `titles.bin` in integers.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// The design's `draw()` (ported as the oracle `m4/titles/mock.ts`) runs on a clock in float seconds. What comes from sines and from its
// small simulation of the shots (the stars' twinkle, the drifting rows of words, the ground's rise, Ben, the enemies and the bubbles) is
// baked per tick by `m4/export-titles.ts`, and so is the fade, whose float rounding at one tick no exact form matches; what is plain
// arithmetic on the clock (the tweens, the drifts of the stars, specks and speed lines) is worked out here in fixed point from constants
// the same file stores. Tick `n` of the loop is `T = n / 60` there, and
// the frames are compared with the oracle's byte for byte (`m4/check-titles.ts`). The draw order is the design's.

use crate::fx::{self, Ease, Tween, ONE};
use crate::ui;
use crate::ui_data::{TITLE_KICKER, TITLE_LINE, TITLE_WORDS};
use crate::video::{self, H, W};

static DATA: &[u8] = include_bytes!("../../../m4/assets/titles.bin");

/// Ticks in one pass of the titles (12.8 s).
pub const LOOP: u32 = 768;

// The sections of `titles.bin`, in the order of its table of offsets.
const MARK: usize = 0;
const TWINKLE: usize = 1;
const GLOBAL: usize = 2;
const ENEMIES: usize = 3;
const BUBBLE_INDEX: usize = 4;
const BUBBLES: usize = 5;
const STATICS: usize = 7;

const STARS: usize = 46;
const SPECKS: usize = 90;
const LINES: usize = 10;
/// Where the specks, the speed lines and the bubble begin in the statics.
const SPECKS_AT: usize = STARS * 10;
const LINES_AT: usize = SPECKS_AT + SPECKS * 10;
const BUBBLE_AT: usize = LINES_AT + LINES * 25;

/// EGA colours by the design's codes.
const BLACK: u8 = 0;
const DARK_GREEN: u8 = 2;
const MAGENTA: u8 = 5;
const BLUE: u8 = 9;
const GREEN: u8 = 10;
const CYAN: u8 = 11;
const RED: u8 = 12;
const PINK: u8 = 13;
const YELLOW: u8 = 14;
const WHITE: u8 = 15;

/// The ticks the design's gates (`T > …`, `T < …` on float seconds) open and close on, as the oracle's floats put them.
const PRESENTS_UNTIL: u32 = 192;
const NAME_FROM: u32 = 43;
const SUBTITLE_FROM: u32 = 91;
const SUBTITLE_UNTIL: u32 = 180;
const WORDS_FROM: u32 = 193;
const KICKER_FROM: u32 = 295;
const LINE_FROM: u32 = 469;
const FLASH_FROM: u32 = 265;
const FLASH_TO: u32 = 268;

// The design's tweens, in milliseconds: `enter` is ease-out-back, `slide` ease-in-out-cubic.
const MARK_IN: Tween = Tween::new(200, 1000, -120, 0, Ease::OutBack);
const MARK_OUT: Tween = Tween::new(2700, 3200, 0, -260, Ease::InOutCubic);
const NAME_IN: Tween = Tween::new(700, 1400, 280, 0, Ease::OutBack);
const NAME_OUT: Tween = Tween::new(2750, 3200, 0, 300, Ease::InOutCubic);
const SUBTITLE_IN: Tween = Tween::new(1500, 2000, 10, 0, Ease::OutBack);
const DROP: Tween = Tween::new(6400, 7200, 60, 0, Ease::InOutCubic);
const LIEUTENANT_IN: Tween = Tween::new(3300, 3900, 360, 0, Ease::OutBack);
const FIZZ_IN: Tween = Tween::new(3800, 4400, -360, 0, Ease::OutBack);
const KICKER_IN: Tween = Tween::new(4900, 5400, -60, 0, Ease::OutBack);
const LINE_IN: Tween = Tween::new(7800, 8300, 6, 0, Ease::OutBack);
const SPEED_LINES: Tween = Tween::new(6800, 7200, 0, 1, Ease::InOutCubic);

/// The scale of the enemies, Ben, the headline text and the rows of words.
const ENEMY_K: i32 = 2;
const BEN_K: i32 = 3;
const TEXT_K: i32 = 3;
const ROW_K: i32 = 5;
/// The enemies in the order they are drawn (the hopper last), and the width of the box the stun's stars centre on.
const ENEMY_NAMES: [[&str; 2]; 7] = [
    ["gloop0", "gloop1"],
    ["beetle0", "beetle1"],
    ["sentry0", "sentry1"],
    ["bat0", "bat1"],
    ["drone0", "drone1"],
    ["phantom0", "phantom1"],
    ["hopper0", "hopper1"],
];
const ENEMY_W: [i32; 7] = [32, 32, 48, 32, 32, 32, 32];

fn u8_at(o: usize) -> u8 {
    DATA[o]
}

fn u16_at(o: usize) -> usize {
    usize::from(DATA[o]) | usize::from(DATA[o + 1]) << 8
}

fn i16_at(o: usize) -> i32 {
    i32::from(i16::from_le_bytes([DATA[o], DATA[o + 1]]))
}

fn i64_at(o: usize) -> i64 {
    let mut b = [0u8; 8];
    b.copy_from_slice(&DATA[o..o + 8]);
    i64::from_le_bytes(b)
}

fn section(n: usize) -> usize {
    u16_at(10 + n * 4) | u16_at(12 + n * 4) << 16
}

/// The first tick with the chase's enemies and bubbles stored.
fn chase() -> u32 {
    u16_at(8) as u32
}

/// One in Q32, the statics' fixed point.
const Q32: i64 = 1 << 32;

/// A pixel at a brightness level, as the design's `put` places one.
fn put(frame: &mut [u8], x: i32, y: i32, colour: u8, level: u8) {
    if x >= 0 && y >= 0 && x < W as i32 && y < H as i32 {
        frame[y as usize * W + x as usize] = level * 16 + colour;
    }
}

/// A span from `x0` up to `x1` on row `y`, as the design's `fill` covers one from a fractional left and width (each end cut toward zero).
fn span(frame: &mut [u8], x0: i32, x1: i32, y: i32, colour: u8, level: u8) {
    ui::fill_level(frame, x0, y, x1 - x0, 1, colour, level);
}

/// The opening titles, playing `loops` times.
pub struct Titles {
    t: u32,
    loops: u8,
    ben: usize,
    enemies: [[usize; 2]; 7],
}

impl Titles {
    pub fn new(loops: u8) -> Titles {
        let id = |n: &str| video::sprite_by_name(n).unwrap_or(usize::MAX);
        Titles { t: 0, loops: loops.max(1), ben: id("ben_shoot"), enemies: ENEMY_NAMES.map(|[a, b]| [id(a), id(b)]) }
    }

    /// The tick of the loop on the screen.
    pub fn now(&self) -> u32 {
        self.t
    }

    /// One tick; false once the last loop has run out. `music` is the titles' song's clock, which the screen will follow to stay in time
    /// with the music; for now the picture keeps its own clock and it is not read.
    pub fn tick(&mut self, music: i32) -> bool {
        let _ = music;
        self.t += 1;
        if self.t >= LOOP {
            self.loops -= 1;
            if self.loops == 0 {
                self.t = LOOP - 1;
                return false;
            }
            self.t = 0;
        }
        true
    }

    /// Draws the whole frame for the current tick.
    pub fn draw(&self, frame: &mut [u8]) {
        let n = self.t;
        let st = section(STATICS);
        let g = section(GLOBAL) + n as usize * 20;
        frame[..W * H].fill(16);

        // Stars, drifting left at 3, 6 or 9 px/s and twinkling. The drift is worked out as x * 60 in Q32, so the tick's own fraction is exact.
        let tw = section(TWINKLE) + n as usize * 23;
        for i in 0..STARS {
            let o = st + i * 10;
            let speed = 3 + (i as i64 % 3) * 3;
            let v = (i64_at(o) * 60 - i64::from(n) * speed * Q32).rem_euclid(330 * 60 * Q32) - 5 * 60 * Q32;
            let level = 6 + (u8_at(tw + i / 2) >> (4 * (i & 1)) & 15);
            put(frame, (v / (60 * Q32)) as i32, i32::from(u8_at(o + 8)), u8_at(o + 9), level);
        }

        // The rows of words drifting left and right, dim.
        let row_level = u8_at(g + 10);
        let gap = ui::text_width("  ") * ROW_K;
        for i in 0..5 {
            let (word, colour) = if i % 2 == 1 { ("Fizz", PINK) } else { ("Lieutenant", BLUE) };
            let y = 2 + i as i32 * 40;
            let mut at = i16_at(g + i * 2);
            for rep in 0..8 {
                at = ui::glyphs_at(frame, at, y, word, colour, ROW_K, row_level);
                if rep < 7 {
                    at += gap;
                }
            }
        }

        // The ground, with specks running left at 150 px/s (x * 2 in Q32), and its green edge.
        let gy = i16_at(g + 12);
        if gy < H as i32 {
            ui::fill_level(frame, 0, gy, W as i32, H as i32 - gy, MAGENTA, 9);
            for i in 0..SPECKS {
                let o = st + SPECKS_AT + i * 10;
                let v = (i64_at(o) * 2 - i64::from(n) * 5 * Q32).rem_euclid(680 * Q32) - 20 * Q32;
                let (x0, x1) = ((v / (2 * Q32)) as i32, ((v + 4 * Q32) / (2 * Q32)) as i32);
                span(frame, x0, x1, gy + 3 + i32::from(u8_at(o + 8)), u8_at(o + 9), 10);
            }
            ui::fill(frame, 0, gy, W as i32, 2, GREEN);
            ui::fill(frame, 0, gy + 2, W as i32, 1, DARK_GREEN);
        }

        // Speed lines, once the chase is under way.
        let sl = SPEED_LINES.q(n);
        if sl > 0 {
            let level = fx::round_q(3 * ONE + 6 * sl) as u8;
            for i in 0..LINES {
                let o = st + LINES_AT + i * 25;
                let (speed, start, len) = (i64_at(o), i64_at(o + 8), i64_at(o + 16));
                let m = (i64::from(n) * speed / 60 + start).rem_euclid(W as i64 * Q32 + len);
                let x = W as i64 * Q32 - m;
                span(frame, (x / Q32) as i32, ((x + len) / Q32) as i32, i32::from(u8_at(o + 24)), CYAN, level);
            }
        }

        if n >= chase() {
            let k = (n - chase()) as usize;
            // The bubbles in flight.
            let bi = section(BUBBLE_INDEX) + k * 2;
            for b in u16_at(bi)..u16_at(bi + 2) {
                let o = section(BUBBLES) + b * 5;
                bubble(frame, i16_at(o), i16_at(o + 2), i32::from(u8_at(o + 4)), st + BUBBLE_AT);
            }
            // The enemies, with the stun's stars over a stunned one.
            for (j, ids) in self.enemies.iter().enumerate() {
                let o = section(ENEMIES) + (k * 7 + j) * 6;
                let (x, y, flags) = (i16_at(o), i16_at(o + 2), u8_at(o + 4));
                video::draw_sprite_scaled(frame, ids[usize::from(flags & 1)], x, y, ENEMY_K, false, flags & 4 != 0);
                if flags & 2 != 0 {
                    stars(frame, i32::from(u8_at(o + 5)), x + (ENEMY_W[j] >> 1) - 8, y - 8);
                }
            }
        }

        // Ben, facing back at the enemies, and his shadow.
        if u8_at(g + 11) != 0 {
            ui::fill_level(frame, i16_at(g + 14) + 6, gy - 1, i32::from(u8_at(g + 18)), 2, BLACK, 1);
            video::draw_sprite_scaled(frame, self.ben, i16_at(g + 14), i16_at(g + 16), BEN_K, true, false);
        }

        // Liminal HQ presents.
        if n < PRESENTS_UNTIL {
            let name = "Liminal HQ";
            let x0 = (W as i32 - (48 + 10 + ui::text_width(name) * TEXT_K)) >> 1;
            mark(frame, x0 + fx::round_q(MARK_IN.q(n) + MARK_OUT.q(n)), 62);
            if n >= NAME_FROM {
                let nx = fx::round_q(i64::from(x0 + 58) * ONE + NAME_IN.q(n) + NAME_OUT.q(n));
                ui::text_big(frame, nx, 74, name, WHITE, TEXT_K);
            }
            if (SUBTITLE_FROM..SUBTITLE_UNTIL).contains(&n) {
                let y = fx::round_q(124 * ONE + SUBTITLE_IN.q(n));
                ui::text(frame, W as i32 / 2 - (ui::text_width("presents") >> 1), y, "presents", YELLOW);
            }
        }

        // The wordmark, kicker and line, as the title menu lays them out.
        if n >= WORDS_FROM {
            let wx = (W as i32 - ui::text_width(TITLE_WORDS) * TEXT_K) >> 1;
            let fizz_at = (ui::text_width("Lieutenant") + 3) * TEXT_K;
            let drop = DROP.at(n);
            ui::text_logo(frame, wx + LIEUTENANT_IN.at(n), 24 + drop, "Lieutenant", WHITE, RED, TEXT_K);
            ui::text_logo(frame, wx + fizz_at + FIZZ_IN.at(n), 24 + drop, "Fizz", WHITE, RED, TEXT_K);
            if n >= KICKER_FROM {
                let cx = W as i32 / 2 + KICKER_IN.at(n);
                ui::text(frame, cx - (ui::text_width(TITLE_KICKER) >> 1), 12 + drop, TITLE_KICKER, YELLOW);
            }
            if n >= LINE_FROM {
                ui::text(frame, W as i32 / 2 - (ui::text_width(TITLE_LINE) >> 1), 56 + LINE_IN.at(n), TITLE_LINE, CYAN);
            }
        }

        fx::fade(frame, u8_at(g + 19));
        if (FLASH_FROM..=FLASH_TO).contains(&n) {
            fx::flash(frame, WHITE);
        }
    }
}

/// The Liminal HQ mark with its top left at (`x`, `y`): its bytes are already `level * 16 + colour`, 0 where clear.
fn mark(frame: &mut [u8], x: i32, y: i32) {
    let o = section(MARK);
    let (w, h) = (usize::from(u8_at(o)), usize::from(u8_at(o + 1)));
    for j in 0..h {
        for i in 0..w {
            let b = u8_at(o + 2 + j * w + i);
            if b != 0 {
                put(frame, x + i as i32, y + j as i32, b & 15, b >> 4);
            }
        }
    }
}

/// A fizz bubble, 8 × 8, every pixel `k` pixels square.
fn bubble(frame: &mut [u8], x: i32, y: i32, k: i32, at: usize) {
    for j in 0..8 {
        for i in 0..8 {
            let c = u8_at(at + j * 8 + i);
            if c != 255 {
                ui::fill(frame, x + i as i32 * k, y + j as i32 * k, k, k, c);
            }
        }
    }
}

/// The stun's three stars turning over an enemy (the enemies' `stars(f)` effect).
fn stars(frame: &mut [u8], f: i32, x: i32, y: i32) {
    for (i, (sx, sy)) in [(2, 3), (8, 1), (13, 4)].into_iter().enumerate() {
        let xx = (sx + f * 3 + i as i32) % 15;
        for (dx, dy) in [(0, 0), (-1, 0), (1, 0), (0, -1), (0, 1)] {
            put(frame, x + xx + dx, y + sy + dy, YELLOW, 14);
        }
        put(frame, x + xx, y + sy, WHITE, 14);
    }
}
