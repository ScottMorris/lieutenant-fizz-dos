// The start-up screen: what the machine has for the game and whether the game can run on it, in the manner of Keen's own.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// The C side finds out about the hardware (the video adapter, input, sound, maths, memory, the speed of a few trial frames)
// and hands over a list of numbers; this shows them, read-only. White with a green tick is found, grey with a cross is
// missing, red with a cross is missing and needed: without a VGA, an x87 or enough memory the game does not start. A
// machine without a VGA cannot show the drawn screen, so the same rows are also written out as text for the text screen.

use crate::ui::{self, Buf, DARK, GREEN, GREY, RED, WHITE, YELLOW};
use crate::video::{H, W};
use core::fmt::Write;

/// Positions in the list of numbers the C side fills in (the `C_*` enum in `m4/c/compat.h`).
pub const VIDEO: usize = 0;
pub const MOUSE: usize = 1;
pub const JOYSTICK: usize = 2;
pub const ADLIB: usize = 3;
pub const BLASTER: usize = 4;
pub const FPU: usize = 5;
/// Free memory in kilobytes, before the game takes its own.
pub const MEMORY_KB: usize = 6;
/// Frames a second of a few trial frames (0 when not measured).
pub const FPS: usize = 7;
pub const COUNT: usize = 8;

/// What is in the `VIDEO` slot: the best adapter found (the `VIDEO_*` enum in `m4/c/compat.h`).
pub const VIDEO_MDA: i32 = 1;
pub const VIDEO_CGA: i32 = 2;
pub const VIDEO_EGA: i32 = 3;
pub const VIDEO_VGA_MONO: i32 = 4;
pub const VIDEO_VGA: i32 = 5;

/// The free memory the game needs, in kilobytes. Starting the game takes about 9.4 MB at once, nearly all of it the sim's
/// instance buffer (120,000 instances of 20 floats, written whole when the world is made), measured as the drop in free
/// physical memory across `fizz_start`; the speed test and play add tens of kilobytes more. 10 MB leaves room for that.
/// With less, CWSDPMI pages to its swap file when it has one, or the allocation fails.
pub const MEMORY_NEEDED_KB: i32 = 10 * 1024;

const PANEL: u8 = 3;
const PANEL_EDGE: u8 = 11;
const TITLE_PANEL: u8 = 5;
const MAGENTA: u8 = 13;
/// Line pitch inside the panels: a glyph is nine rows (capitals seven, descenders two), so lines touch but do not overlap.
const ROW: i32 = 9;
const PANELS: [&str; 4] = ["Video", "Input", "Sound", "Machine"];

#[derive(Clone, Copy, PartialEq)]
enum State {
    Found,
    Missing,
    /// Missing, and the game cannot run without it.
    Needed,
}

fn has(on: bool) -> State {
    if on {
        State::Found
    } else {
        State::Missing
    }
}

fn need(on: bool) -> State {
    if on {
        State::Found
    } else {
        State::Needed
    }
}

fn vga(v: &[i32]) -> bool {
    v[VIDEO] >= VIDEO_VGA_MONO
}

fn memory_ok(v: &[i32]) -> bool {
    v[MEMORY_KB] >= MEMORY_NEEDED_KB
}

/// Whether the game can start: a VGA, an x87 and enough free memory.
pub fn ready(v: &[i32]) -> bool {
    vga(v) && v[FPU] != 0 && memory_ok(v)
}

/// Calls `f` with each row of the four panels: the panel, the row in it, the words and the state.
fn rows(v: &[i32], cpu: &str, mut f: impl FnMut(usize, i32, &str, State)) {
    let on = vga(v);
    f(0, 0, "VGA adapter", has(on));
    // Without a VGA the mode row names the best the machine has, which is not enough.
    let mode = match v[VIDEO] {
        VIDEO_VGA_MONO => "Mono VGA 320x200x256",
        VIDEO_EGA => "EGA 640x350x16",
        VIDEO_CGA => "CGA 640x200x2",
        VIDEO_MDA => "80x25 text, mono",
        _ => "VGA 320x200x256",
    };
    f(0, 1, mode, need(on));
    // The shades are the palette trick, so they come with the VGA rather than being a probe of their own.
    f(0, 2, "14 lighting shades", has(on));
    f(1, 0, "Keyboard", State::Found);
    f(1, 1, "Mouse", has(v[MOUSE] != 0));
    f(1, 2, "Joystick", has(v[JOYSTICK] != 0));
    f(2, 0, "PC Speaker", State::Found);
    f(2, 1, "AdLib", has(v[ADLIB] != 0));
    f(2, 2, "Sound Blaster", has(v[BLASTER] != 0));
    let mut line = Buf::new();
    let _ = write!(line, "Processor {cpu}");
    f(3, 0, line.as_str(), State::Found);
    f(3, 1, "x87 maths (needed)", need(v[FPU] != 0));
    let mut line = Buf::new();
    let mb = v[MEMORY_KB] / 1024;
    if memory_ok(v) {
        let _ = write!(line, "{mb} MB memory free");
    } else {
        let _ = write!(line, "{mb} MB free, needs {}", MEMORY_NEEDED_KB / 1024);
    }
    f(3, 2, line.as_str(), need(memory_ok(v)));
}

/// The footer's speed line: the words before the verdict, the verdict and its colour.
fn speed(v: &[i32]) -> (Buf, &'static str, u8) {
    let mut line = Buf::new();
    let fps = v[FPS];
    if !ready(v) {
        let _ = line.write_str("Speed test: not run");
        return (line, "", GREY);
    }
    if fps == 0 {
        return (line, "", GREY);
    }
    let (verdict, colour) = if fps >= 90 {
        ("smooth", GREEN)
    } else if fps >= 62 {
        ("fine", GREEN)
    } else if fps >= 40 {
        ("borderline", YELLOW)
    } else {
        ("too slow", RED)
    };
    let _ = write!(line, "Speed test: {fps} frames a second - ");
    (line, verdict, colour)
}

/// The footer's last line: the call to play, or what is missing. `short` names the x87 more briefly.
fn call(v: &[i32], short: bool) -> (Buf, u8) {
    let mut line = Buf::new();
    if ready(v) {
        let _ = line.write_str("Press Enter to play, Esc to quit");
        return (line, YELLOW);
    }
    let mut parts: [&str; 3] = [""; 3];
    let mut n = 0;
    for (missing, name) in [
        (!vga(v), "VGA"),
        (v[FPU] == 0, if short { "x87" } else { "x87 maths" }),
        (!memory_ok(v), "more memory"),
    ] {
        if missing {
            parts[n] = name;
            n += 1;
        }
    }
    let _ = line.write_str("Needs ");
    for (i, part) in parts[..n].iter().enumerate() {
        let gap = if i == 0 {
            ""
        } else if i + 1 == n {
            " and "
        } else {
            ", "
        };
        let _ = write!(line, "{gap}{part}");
    }
    let _ = line.write_str(": Esc to quit");
    (line, RED)
}

/// Draws the whole screen into `frame`; `v` is the list of numbers and `cpu` the processor's family code.
pub fn draw(frame: &mut [u8], v: &[i32], cpu: &str) {
    // A blue weave, like the backdrop of Keen's.
    for y in 0..H {
        for x in 0..W {
            frame[y * W + x] = ui::FULL + if (x + y) % 2 == 0 { 1 } else { 9 };
        }
    }
    let title = ui::Panel { x: 24, y: 4, w: 272, h: 44 }.draw(frame, TITLE_PANEL, MAGENTA);
    let w = ui::text_width("Lieutenant Fizz") * 2;
    ui::text_big(frame, (W as i32 - w) / 2, title.top(false) - 1, "Lieutenant Fizz", WHITE, 2);
    ui::text_centred(frame, W as i32 / 2, title.top(false) + 19, "Episode 1 - The Cocoa Caper", YELLOW);

    // Two equal columns: Video and Input above, Sound and Machine below.
    let (pw, ph) = (148, 46);
    let at = |panel: usize| (if panel % 2 == 0 { 8 } else { 164 }, if panel < 2 { 52 } else { 102 });
    for (i, name) in PANELS.iter().enumerate() {
        let (x, y) = at(i);
        let p = ui::Panel { x, y, w: pw, h: ph }.draw(frame, PANEL, PANEL_EDGE);
        p.legend(frame, name, PANEL, YELLOW);
    }
    let max = pw - ui::Panel::SIDE * 2;
    rows(v, cpu, |panel, n, label, state| {
        let (x, y) = at(panel);
        let (x, y) = (x + ui::Panel::SIDE, y + ui::Panel::TOP_LEGEND + n * ROW);
        match state {
            State::Found => {
                ui::tick_mark(frame, x, y, GREEN);
                ui::text_fit(frame, x + 11, y, label, WHITE, max - 11);
            }
            State::Missing | State::Needed => {
                let colour = if state == State::Needed { RED } else { DARK };
                // The cross is six pixels to the tick's eight, so it sits one pixel in to share the tick's centre.
                ui::cross_mark(frame, x + 1, y, colour);
                ui::text_fit(frame, x + 11, y, label, colour, max - 11);
            }
        }
    });

    // The footer: the speed test, then the call to play or what is missing.
    let foot = ui::Panel { x: 24, y: 152, w: 272, h: 44 }.draw(frame, TITLE_PANEL, MAGENTA);
    let (fx, fw, y0) = (foot.left(), foot.inner_w(), foot.top(false));
    let (line, verdict, colour) = speed(v);
    let start = (W as i32 - ui::text_width(line.as_str()) - ui::text_width(verdict)) / 2;
    let end = ui::text_fit(frame, start, y0 + 4, line.as_str(), if verdict.is_empty() { colour } else { WHITE }, fw);
    ui::text_fit(frame, end, y0 + 4, verdict, colour, fx + fw - end);
    let (mut line, colour) = call(v, false);
    if ui::text_width(line.as_str()) > fw {
        line = call(v, true).0;
    }
    let cut = (W as i32 - ui::text_width(line.as_str()).min(fw)) / 2;
    ui::text_fit(frame, cut, y0 + 16, line.as_str(), colour, fw);
}

/// Colour codes in the text form of the screen: each byte below 8 sets the colour of what follows it.
const T_WHITE: u8 = 1;
const T_GREY: u8 = 2;
const T_RED: u8 = 3;
const T_GREEN: u8 = 4;
const T_YELLOW: u8 = 5;
/// The tick in code page 437.
const TICK: u8 = 0xFB;
/// Where the right-hand column starts on an 80-column text screen.
const COLUMN: usize = 40;

/// Bytes going into the C side's buffer; what does not fit is dropped.
struct Out<'a> {
    bytes: &'a mut [u8],
    len: usize,
    /// Characters on the current line, not counting colour codes.
    col: usize,
}

impl Out<'_> {
    fn byte(&mut self, b: u8) {
        if self.len < self.bytes.len() {
            self.bytes[self.len] = b;
            self.len += 1;
        }
        if b == b'\n' {
            self.col = 0;
        } else if b >= 8 {
            self.col += 1;
        }
    }
    fn colour(&mut self, code: u8) {
        self.byte(code);
    }
    fn text(&mut self, s: &str) {
        for b in s.bytes() {
            self.byte(b);
        }
    }
    fn pad_to(&mut self, col: usize) {
        while self.col < col {
            self.byte(b' ');
        }
    }
}

fn code(colour: u8) -> u8 {
    match colour {
        GREEN => T_GREEN,
        RED => T_RED,
        YELLOW => T_YELLOW,
        GREY | DARK => T_GREY,
        _ => T_WHITE,
    }
}

/// Writes the screen as lines of text for the 80x25 text screen into `out`, with colour codes (see `T_WHITE`); returns
/// the number of bytes written.
pub fn text(out: &mut [u8], v: &[i32], cpu: &str) -> usize {
    let mut o = Out { bytes: out, len: 0, col: 0 };
    o.colour(T_WHITE);
    o.text("\n  Lieutenant Fizz");
    o.colour(T_YELLOW);
    o.text(" - Episode 1 - The Cocoa Caper\n");
    // Each row of panels in two columns: the legends, then three rows.
    let mut grid: [[(Buf, State); 3]; 4] = core::array::from_fn(|_| core::array::from_fn(|_| (Buf::new(), State::Missing)));
    rows(v, cpu, |panel, n, label, state| {
        let _ = grid[panel][n as usize].0.write_str(label);
        grid[panel][n as usize].1 = state;
    });
    for pair in [0, 2] {
        o.text("\n");
        o.colour(T_YELLOW);
        o.pad_to(2);
        o.text(PANELS[pair]);
        o.pad_to(COLUMN);
        o.text(PANELS[pair + 1]);
        o.text("\n");
        for n in 0..3 {
            for (side, panel) in [pair, pair + 1].into_iter().enumerate() {
                o.pad_to(if side == 0 { 2 } else { COLUMN });
                let (label, state) = &grid[panel][n];
                match state {
                    State::Found => {
                        o.colour(T_GREEN);
                        o.byte(TICK);
                        o.colour(T_WHITE);
                    }
                    State::Missing => {
                        o.colour(T_GREY);
                        o.byte(b'x');
                    }
                    State::Needed => {
                        o.colour(T_RED);
                        o.byte(b'x');
                    }
                }
                o.byte(b' ');
                o.text(label.as_str());
            }
            o.text("\n");
        }
    }
    o.text("\n");
    let (line, verdict, colour) = speed(v);
    if !line.as_str().is_empty() {
        o.pad_to(2);
        o.colour(if verdict.is_empty() { code(colour) } else { T_WHITE });
        o.text(line.as_str());
        o.colour(code(colour));
        o.text(verdict);
        o.text("\n");
    }
    let (line, colour) = call(v, false);
    o.pad_to(2);
    o.colour(code(colour));
    o.text(line.as_str());
    o.text("\n");
    o.colour(T_WHITE);
    o.len
}
