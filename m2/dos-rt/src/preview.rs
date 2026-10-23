// The start-up screen: what the machine has for the game and whether the game can run on it, in the manner of Keen's own.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// The C side finds out about the hardware (the video adapter, input, sound, maths, memory, the speed of a few trial frames)
// and hands over a list of numbers; this shows them, read-only. White with a green tick is found, grey with a cross is
// missing, yellow with a triangle is there but short of the best, red with a cross is missing and needed: without a VGA,
// an x87 or the least memory the game does not start. The same rows also make an 80x25 text screen (`cells`), which a
// machine without a VGA can show and which the `text` option asks for on any machine.

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

/// The free memory that runs the game from memory alone, in kilobytes. Starting the game takes about 9.4 MB at once, nearly
/// all of it the sim's instance buffer (120,000 instances of 20 floats, written whole when the world is made), measured in
/// DOSBox as the drop in free physical memory across `fizz_start` (13,648 KB free before, 4,224 KB after); the speed test
/// and play add tens of kilobytes more. 10 MB leaves room for that.
pub const MEMORY_BEST_KB: i32 = 10 * 1024;
/// The free memory below which the game does not start, in kilobytes. Below `MEMORY_BEST_KB` CWSDPMI pages what does not
/// fit to its swap file on disk: the game runs, but the start is slow and so is any frame that touches paged-out memory,
/// and with no swap file (no writable drive) the allocation fails. In DOSBox, with sound, the game ran that way with
/// `memsize = 4` (1.3 MB free before the start); with `memsize = 2` the sound handler met a paged-out page and DOSBox
/// stopped ("Pagefault didn't correct page"), and with `memsize = 1` the program ended without its report. Those two read
/// under 1 MB here (the count wraps round, which `compat_probe` reads as none), so 1 MB is where the game stops working.
pub const MEMORY_FLOOR_KB: i32 = 1024;

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
    /// Found, but short of the best: the game runs, slowly.
    Warn,
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
    v[MEMORY_KB] >= MEMORY_FLOOR_KB
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
    let state = if v[MEMORY_KB] >= MEMORY_BEST_KB {
        let _ = write!(line, "{mb} MB memory free");
        State::Found
    } else if memory_ok(v) {
        let _ = write!(line, "{mb} MB free, {} is best", MEMORY_BEST_KB / 1024);
        State::Warn
    } else {
        let _ = write!(line, "Under {} MB free", MEMORY_FLOOR_KB / 1024);
        State::Needed
    };
    f(3, 2, line.as_str(), state);
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

/// What the game needs and the machine lacks, joined as in a sentence ("VGA, x87 and more memory"). `short` names the x87
/// more briefly.
fn needs(v: &[i32], short: bool, line: &mut Buf) {
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
}

/// The footer's last line: the call to play, or what is missing. `short` names the x87 more briefly.
fn call(v: &[i32], short: bool) -> (Buf, u8) {
    let mut line = Buf::new();
    if ready(v) {
        let _ = line.write_str("Press Enter to play, Esc to quit");
        return (line, YELLOW);
    }
    let _ = line.write_str("Needs ");
    needs(v, short, &mut line);
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
            State::Warn => {
                ui::warn_mark(frame, x, y, YELLOW);
                ui::text_fit(frame, x + 11, y, label, YELLOW, max - 11);
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


/// Frames a second in the `FPS` slot while the speed test runs: the text screen is up before the test and says so.
pub const FPS_RUNNING: i32 = -1;

/// The text screen's size in character cells.
pub const COLS: usize = 80;
pub const ROWS: usize = 25;
pub const CELLS: usize = COLS * ROWS;

/// Text-mode colours (the attribute's nibbles). A background is 0..7 only: bit 7 of the attribute is never set, so it
/// cannot blink whatever the adapter makes of it.
const T_BLACK: u8 = 0;
const T_RED: u8 = 4;
const T_BROWN: u8 = 6;
const T_LIGHT_GREEN: u8 = 10;
const T_LIGHT_CYAN: u8 = 11;
const T_YELLOW: u8 = 14;
const T_WHITE: u8 = 15;

/// Code page 437 characters: the tick, the warning triangle, two shades and the double-line box.
const CP_TICK: u8 = 0xFB;
const CP_WARN: u8 = 0x1E;
const CP_LIGHT_SHADE: u8 = 0xB0;
const CP_DARK_SHADE: u8 = 0xB2;
const CP_H: u8 = 0xCD;
const CP_V: u8 = 0xBA;
const CP_TOP_LEFT: u8 = 0xC9;
const CP_TOP_RIGHT: u8 = 0xBB;
const CP_BOTTOM_LEFT: u8 = 0xC8;
const CP_BOTTOM_RIGHT: u8 = 0xBC;

/// The text screen being built: a cell is the character in the low byte and the attribute in the high byte (foreground in
/// the low nibble, background in bits 4..6), as text-mode video memory holds it.
struct Cells<'a>(&'a mut [u16]);

impl Cells<'_> {
    fn set(&mut self, x: usize, y: usize, ch: u8, fg: u8, bg: Option<u8>) {
        if x < COLS && y < ROWS {
            let cell = &mut self.0[y * COLS + x];
            let bg = bg.unwrap_or((*cell >> 12) as u8 & 7);
            *cell = u16::from(ch) | u16::from(fg & 15) << 8 | u16::from(bg & 7) << 12;
        }
    }

    /// Writes `s` from column `x`; with no `bg` each cell keeps the background it had. Returns the column after it.
    fn put(&mut self, x: usize, y: usize, s: &[u8], fg: u8, bg: Option<u8>) -> usize {
        for (i, &ch) in s.iter().enumerate() {
            self.set(x + i, y, ch, fg, bg);
        }
        x + s.len()
    }

    fn fill(&mut self, x: usize, y: usize, n: usize, ch: u8, fg: u8, bg: Option<u8>) {
        for i in 0..n {
            self.set(x + i, y, ch, fg, bg);
        }
    }

    /// Changes the background of `n` cells and leaves their characters and foregrounds alone: the bar behind a row.
    fn recolour(&mut self, x: usize, y: usize, n: usize, bg: u8) {
        for i in 0..n {
            if x + i < COLS && y < ROWS {
                let cell = &mut self.0[y * COLS + x + i];
                *cell = *cell & 0x8FFF | u16::from(bg & 7) << 12;
            }
        }
    }

    fn centre(&mut self, y: usize, s: &[u8], fg: u8) {
        self.put(COLS.saturating_sub(s.len()) / 2, y, s, fg, None);
    }

    /// A plate: a black or coloured fill with a double-line edge, a legend on the top edge if `title` is given, and a shadow
    /// of light shade in brown two columns wide down the right (a cell is about twice as tall as it is wide) and one row
    /// below, offset two columns.
    fn plate(&mut self, x: usize, y: usize, w: usize, h: usize, fill: u8, edge: u8, title: Option<&str>) {
        for r in 0..h {
            self.fill(x, y + r, w, b' ', edge, Some(fill));
        }
        self.set(x, y, CP_TOP_LEFT, edge, Some(fill));
        self.fill(x + 1, y, w - 2, CP_H, edge, Some(fill));
        self.set(x + w - 1, y, CP_TOP_RIGHT, edge, Some(fill));
        for r in 1..h - 1 {
            self.set(x, y + r, CP_V, edge, Some(fill));
            self.set(x + w - 1, y + r, CP_V, edge, Some(fill));
        }
        self.set(x, y + h - 1, CP_BOTTOM_LEFT, edge, Some(fill));
        self.fill(x + 1, y + h - 1, w - 2, CP_H, edge, Some(fill));
        self.set(x + w - 1, y + h - 1, CP_BOTTOM_RIGHT, edge, Some(fill));
        if let Some(title) = title {
            let end = self.put(x + 2, y, b" ", T_WHITE, Some(T_BROWN));
            let end = self.put(end, y, title.as_bytes(), T_WHITE, Some(T_BROWN));
            self.put(end, y, b" ", T_WHITE, Some(T_BROWN));
        }
        for r in 1..=h {
            self.fill(x + w, y + r, 2, CP_LIGHT_SHADE, T_BROWN, Some(T_BLACK));
        }
        self.fill(x + 2, y + h, w, CP_LIGHT_SHADE, T_BROWN, Some(T_BLACK));
    }
}

/// Builds the start-up screen as 80x25 text cells into `out` (`CELLS` of them): the same rows and rules as `draw`, in brown,
/// yellow and white on black over a field of dark shade. `v[FPS]` is `FPS_RUNNING` while the speed test runs.
pub fn cells(out: &mut [u16], v: &[i32], cpu: &str) {
    let mut s = Cells(out);
    for cell in s.0.iter_mut() {
        *cell = u16::from(CP_DARK_SHADE) | u16::from(T_BROWN) << 8;
    }
    // A narrow title plate, four panels across the full width in two columns, and a narrow footer.
    s.plate(6, 2, 68, 3, T_BROWN, T_YELLOW, None);
    s.centre(3, b"Lieutenant Fizz - Episode 1 - The Cocoa Caper", T_WHITE);
    const PANEL_X: [usize; 2] = [2, 41];
    const PANEL_Y: [usize; 2] = [6, 12];
    const PANEL_W: usize = 37;
    const FOOT_Y: usize = 18;
    for (i, name) in PANELS.iter().enumerate() {
        s.plate(PANEL_X[i % 2], PANEL_Y[i / 2], PANEL_W, 5, T_BLACK, T_YELLOW, Some(name));
    }
    rows(v, cpu, |panel, n, label, state| {
        let (x, y) = (PANEL_X[panel % 2], PANEL_Y[panel / 2] + 1 + n as usize);
        let (mark, mark_fg, label_fg, bar) = match state {
            State::Found => (CP_TICK, T_LIGHT_GREEN, T_WHITE, None),
            State::Missing => (b'x', T_BROWN, T_BROWN, None),
            State::Warn => (CP_WARN, T_YELLOW, T_YELLOW, Some(T_BROWN)),
            State::Needed => (b'!', T_WHITE, T_WHITE, Some(T_RED)),
        };
        if let Some(bar) = bar {
            s.recolour(x + 1, y, PANEL_W - 2, bar);
        }
        s.set(x + 2, y, mark, mark_fg, None);
        s.put(x + 4, y, label.as_bytes(), label_fg, None);
    });

    s.plate(6, FOOT_Y, 68, 5, T_BLACK, T_YELLOW, None);
    if !ready(v) {
        s.centre(FOOT_Y + 1, b"Speed test: not run", T_BROWN);
        s.recolour(7, FOOT_Y + 3, 66, T_RED);
        let mut line = Buf::new();
        let _ = line.write_str("! Needs ");
        needs(v, true, &mut line);
        let _ = line.write_str(": Esc to quit");
        s.centre(FOOT_Y + 3, line.as_str().as_bytes(), T_WHITE);
        return;
    }
    let fps = v[FPS];
    if fps < 0 {
        s.centre(FOOT_Y + 1, b"Speed test: running", T_BROWN);
        return;
    }
    let (verdict, fg, bg) = if fps >= 90 {
        ("smooth", T_LIGHT_GREEN, None)
    } else if fps >= 62 {
        ("fine", T_LIGHT_CYAN, None)
    } else if fps >= 40 {
        ("borderline", T_YELLOW, None)
    } else {
        ("too slow", T_WHITE, Some(T_RED))
    };
    let mut head = Buf::new();
    let _ = write!(head, "Speed test: {fps} frames a second - ");
    let x = COLS.saturating_sub(head.as_str().len() + verdict.len()) / 2;
    let x = s.put(x, FOOT_Y + 1, head.as_str().as_bytes(), T_BROWN, None);
    s.put(x, FOOT_Y + 1, verdict.as_bytes(), fg, bg);
    s.centre(FOOT_Y + 3, b"Press Enter to play, Esc to quit", T_YELLOW);
}
