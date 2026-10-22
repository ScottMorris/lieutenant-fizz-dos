// The story screens: the letterboxed panels of the ending, and the credits roll.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Both are presentation only and follow the main game's `BeatCursor` and `CreditsRoll`: a scene's text arrives a page at a time, as
// many of its beats as fit the text box, typed at 83 characters a second; a press completes the page being typed, the next one moves
// on. The roll scrolls at 40 pixels a second; a press of Enter, Jump or Fire switches it to 4 times that and back, and it stops with the
// closing card in the middle and waits for a press.

use core::fmt::Write;

use crate::ui::{self, Buf};
use crate::ui_data::{CREDITS, CREDITS_RETURN, CREDITS_SUBTITLE, CREDITS_THANKS, CREDITS_THANKS_LINE, CREDITS_TITLE};
use crate::video::{H, W};

/// A scene is the place it happens and its beats of text.
pub type Scenes = &'static [(&'static str, &'static [&'static str])];

/// The text box's width in pixels and the most lines a page may take.
const TEXT_W: i32 = 288;
const PAGE_LINES: usize = 4;
/// Sixteenths of a character typed a tick: 83 a second at 60 ticks a second.
const TYPE_STEP: u32 = 22;

/// What a press did.
#[derive(Clone, Copy, PartialEq)]
pub enum Press {
    /// The page was still being typed and is now whole.
    Complete,
    /// On to the next page of the scene.
    Page,
    /// On to the next scene.
    Scene,
    /// The last page of the last scene was already complete.
    End,
}

/// Where the player is in a sequence of scenes, and how much of the page is typed.
pub struct Reel {
    scenes: Scenes,
    scene: usize,
    /// The first beat of the page showing and how many beats it holds.
    first: usize,
    count: usize,
    typed16: u32,
    /// The characters on the page showing, counted when the page changes.
    len: u32,
}

impl Reel {
    pub const fn empty() -> Reel {
        Reel { scenes: &[], scene: 0, first: 0, count: 0, typed16: 0, len: 0 }
    }

    /// Begins at the first page of `scenes`.
    pub fn start(&mut self, scenes: Scenes) {
        self.scenes = scenes;
        self.scene = 0;
        self.first = 0;
        self.count = self.fit(0);
        self.typed16 = 0;
        self.len = self.page_chars();
    }

    /// The scene number the sequence is at.
    pub fn scene(&self) -> usize {
        self.scene
    }

    fn beats(&self) -> &'static [&'static str] {
        self.scenes.get(self.scene).map_or(&[], |s| s.1)
    }

    /// The text of the beats `first..first + count`, joined by spaces.
    fn join(&self, first: usize, count: usize, out: &mut Buf) {
        for (i, beat) in self.beats().iter().enumerate().skip(first).take(count) {
            if i > first {
                let _ = out.write_str(" ");
            }
            let _ = out.write_str(beat);
        }
    }

    /// How many beats from `first` fit the text box.
    fn fit(&self, first: usize) -> usize {
        let left = self.beats().len().saturating_sub(first);
        let mut count = 1.min(left);
        while count < left {
            let mut text = Buf::new();
            self.join(first, count + 1, &mut text);
            let mut lines = [(0, 0); PAGE_LINES + 1];
            if ui::wrap(text.as_str(), TEXT_W, &mut lines) > PAGE_LINES {
                break;
            }
            count += 1;
        }
        count
    }

    fn page(&self) -> Buf {
        let mut text = Buf::new();
        self.join(self.first, self.count, &mut text);
        text
    }

    fn page_chars(&self) -> u32 {
        self.page().as_str().chars().count() as u32
    }

    /// Types a little more of the page.
    pub fn tick(&mut self) {
        self.typed16 = (self.typed16 + TYPE_STEP).min(self.len * 16);
    }

    /// One press of Enter, Jump or Fire.
    pub fn press(&mut self) -> Press {
        if self.typed16 < self.len * 16 {
            self.typed16 = self.len * 16;
            return Press::Complete;
        }
        if self.first + self.count < self.beats().len() {
            self.first += self.count;
            self.count = self.fit(self.first);
            self.typed16 = 0;
            self.len = self.page_chars();
            return Press::Page;
        }
        if self.scene + 1 < self.scenes.len() {
            self.scene += 1;
            self.first = 0;
            self.count = self.fit(0);
            self.typed16 = 0;
            self.len = self.page_chars();
            return Press::Scene;
        }
        Press::End
    }

    /// The letterbox: a bar at the top with the place and a dot for each scene, and the typed page in a bar at the foot. What is
    /// under the bars is dimmed; `art` says whether the caller has drawn a picture there (and so wants it clear).
    pub fn draw(&self, frame: &mut [u8], tick: u32, art: bool) {
        let (w, h) = (W as i32, H as i32);
        if !art {
            ui::dim(frame, 0, 0, w, h, 11);
        }
        ui::fill(frame, 0, 0, w, 26, ui::BLACK);
        ui::fill(frame, 0, h - 62, w, 62, ui::BLACK);
        ui::fill(frame, 0, 26, w, 1, ui::DARK);
        ui::fill(frame, 0, h - 63, w, 1, ui::DARK);
        let place = self.scenes.get(self.scene).map_or("", |s| s.0);
        ui::text_centred(frame, w / 2, 6, place, ui::YELLOW);
        // A filled square for each scene reached and an open one for each to come.
        let n = self.scenes.len() as i32;
        let x0 = (w - n * 10 + 4) / 2;
        for i in 0..n {
            if i as usize <= self.scene {
                ui::fill(frame, x0 + i * 10, 18, 6, 4, ui::WHITE);
            } else {
                ui::outline(frame, x0 + i * 10, 18, 6, 4, ui::GREY);
            }
        }
        let text = self.page();
        let typed = (self.typed16 / 16) as usize;
        ui::paragraph_upto(frame, 16, h - 54, TEXT_W, text.as_str(), ui::WHITE, typed);
        if self.typed16 >= self.len * 16 && (tick / 20) % 2 == 0 {
            ui::text(frame, w - 16 - ui::text_width("Enter"), h - 14, "Enter", ui::CYAN);
        }
    }
}

/// Fixed point for the roll's offset: 65536 to the pixel, so 40 pixels a second at 60 ticks a second does not lose a fraction a tick.
const ROLL_ONE: u32 = 65536;
const ROLL_STEP: u32 = 40 * ROLL_ONE / 60;
const ROLL_FAST: u32 = 4;

/// The credits roll.
pub struct Roll {
    offset: u32,
    /// How far the roll travels to the closing card, found when it starts.
    total: u32,
    fast: bool,
    /// Ticks since the roll came to rest on the closing card.
    held_for: u32,
}

impl Roll {
    pub const fn new() -> Roll {
        Roll { offset: 0, total: 0, fast: false, held_for: 0 }
    }

    pub fn start(&mut self) {
        let (close, _) = walk(None, 0);
        *self = Roll::new();
        self.total = ((H as i32 / 2 + close + 30).max(0) as u32) * ROLL_ONE;
    }

    /// Whether the roll has stopped on the closing card.
    pub fn held(&self) -> bool {
        self.offset >= self.total
    }

    pub fn tick(&mut self) {
        if self.held() {
            self.held_for += 1;
            return;
        }
        self.offset = (self.offset + ROLL_STEP * if self.fast { ROLL_FAST } else { 1 }).min(self.total);
    }

    /// A press of Enter, Jump or Fire: speeds the roll up or slows it down; on the closing card it leaves. Returns whether to leave.
    pub fn press(&mut self) -> bool {
        if self.held() {
            return true;
        }
        self.fast = !self.fast;
        false
    }

    pub fn draw(&self, frame: &mut [u8]) {
        let (w, h) = (W as i32, H as i32);
        ui::fill(frame, 0, 0, w, h, ui::BLACK);
        let (_, ret_y) = walk(Some(frame), (self.offset / ROLL_ONE) as i32);
        let hint = if self.held() {
            // The last line comes in once the roll has stopped.
            let sy = h - (self.offset / ROLL_ONE) as i32 + ret_y;
            let colour = if self.held_for > 30 { ui::WHITE } else { ui::DARK };
            if self.held_for > 10 {
                ui::text_centred(frame, w / 2, sy, CREDITS_RETURN, colour);
            }
            "Enter continues"
        } else if self.fast {
            "Enter slows down, Esc skips"
        } else {
            "Enter speeds up, Esc skips"
        };
        ui::fill(frame, 0, h - 14, w, 14, ui::BLACK);
        ui::text_centred(frame, w / 2, h - 12, hint, ui::GREY);
    }
}

/// Lays the roll out from its first line, which sits `offset` pixels above the foot of the screen at the start; draws it when `frame`
/// is given. Returns where the closing card starts and where its last line goes, in pixels from the first line.
fn walk(mut frame: Option<&mut [u8]>, offset: i32) -> (i32, i32) {
    let (w, h) = (W as i32, H as i32);
    let cx = w / 2;
    let base = h - offset;
    let mut y = 0;
    let put = |frame: &mut Option<&mut [u8]>, y: i32, f: &dyn Fn(&mut [u8], i32)| {
        let sy = base + y;
        if sy > -24 && sy < h {
            if let Some(fr) = frame.as_deref_mut() {
                f(fr, sy);
            }
        }
    };
    put(&mut frame, y, &|fr, sy| {
        let tw = ui::text_width(CREDITS_TITLE) * 2;
        ui::text_logo(fr, cx - tw / 2, sy, CREDITS_TITLE, ui::WHITE, ui::RED, 2);
    });
    y += 32;
    put(&mut frame, y, &|fr, sy| {
        ui::text_centred(fr, cx, sy, CREDITS_SUBTITLE, ui::CYAN);
    });
    y += 48;
    for &(head, lines) in CREDITS {
        put(&mut frame, y, &|fr, sy| {
            ui::text_centred(fr, cx, sy, head, ui::YELLOW);
        });
        y += 18;
        for &(role, name) in lines {
            put(&mut frame, y, &|fr, sy| {
                ui::text_centred(fr, cx, sy, role, ui::GREY);
                ui::text_centred(fr, cx, sy + 10, name, ui::WHITE);
            });
            y += 26;
        }
        y += 28;
    }
    let close = y;
    put(&mut frame, y, &|fr, sy| {
        let tw = ui::text_width(CREDITS_THANKS) * 2;
        ui::text_big(fr, cx - tw / 2, sy, CREDITS_THANKS, ui::WHITE, 2);
    });
    y += 26;
    put(&mut frame, y, &|fr, sy| {
        ui::text_centred(fr, cx, sy, CREDITS_THANKS_LINE, ui::CYAN);
    });
    y += 30;
    (close, y)
}
