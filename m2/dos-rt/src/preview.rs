// The start-up screen: what the machine has and whether the demo will run well on it, in the manner of Keen's own.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// The C side finds out about the hardware (video, input, sound, memory, the speed of a few trial frames) and hands over a
// list of numbers; this draws them.

use crate::ui::{self, CYAN, DARK, GREEN, GREY, RED, WHITE, YELLOW};
use crate::video::{H, W};

/// Positions in the list of numbers the C side fills in.
pub const VGA: usize = 0;
pub const EGA: usize = 1;
pub const MOUSE: usize = 2;
pub const JOYSTICK: usize = 3;
pub const ADLIB: usize = 4;
pub const BLASTER: usize = 5;
pub const FPU: usize = 6;
/// Free memory in kilobytes.
pub const MEMORY_KB: usize = 7;
/// Frames a second of a few trial frames (0 when not measured).
pub const FPS: usize = 8;
/// The sound mode chosen: 0 auto, 1 AdLib, 2 PC speaker, 3 off, then what that will actually be.
pub const SOUND_CHOICE: usize = 9;
pub const SOUND_USING: usize = 10;
/// Where the game's clock ticks, or 0: whether the speed test could run.
pub const COUNT: usize = 12;

const PANEL: u8 = 3;
const PANEL_EDGE: u8 = 11;
const TITLE_PANEL: u8 = 5;
const MAGENTA: u8 = 13;
/// Line pitch inside the panels: a glyph is nine rows (capitals seven, descenders two), so lines touch but do not overlap.
const ROW: i32 = 9;

/// A line with a tick or a cross: wanted and found is white, not found is dim. It never runs past `max` pixels.
fn item(frame: &mut [u8], x: i32, y: i32, max: i32, label: &str, found: bool, needed: bool) {
    if found {
        ui::tick_mark(frame, x, y, GREEN);
        ui::text_fit(frame, x + 11, y, label, WHITE, max - 11);
    } else {
        ui::cross_mark(frame, x, y, if needed { RED } else { DARK });
        ui::text_fit(frame, x + 11, y, label, if needed { RED } else { DARK }, max - 11);
    }
}

/// Draws the whole screen into `frame`; `v` is the list of numbers and `cpu` the processor's name.
pub fn draw(frame: &mut [u8], v: &[i32], cpu: &str) {
    // A blue weave, like the backdrop of Keen's.
    for y in 0..H {
        for x in 0..W {
            frame[y * W + x] = ui::FULL + if (x + y) % 2 == 0 { 1 } else { 9 };
        }
    }
    // The title, with the port's name on the top edge.
    let title = ui::Panel { x: 24, y: 4, w: 272, h: 44 }.draw(frame, TITLE_PANEL, MAGENTA);
    title.legend(frame, "Liminal HQ DOS port - work in progress", TITLE_PANEL, CYAN);
    let w = ui::text_width("Lieutenant Fizz") * 2;
    ui::text_big(frame, (W as i32 - w) / 2, title.top(true) - 1, "Lieutenant Fizz", WHITE, 2);
    ui::text_centred(frame, W as i32 / 2, title.top(true) + 21, "Episode 1 - The Cocoa Caper", YELLOW);

    let (lx, rx, pw, ph) = (8, 164, 148, 46);
    let (row1, row2) = (52, 102);
    let panels = [
        (lx, row1, "Video"),
        (rx, row1, "Input"),
        (lx, row2, "Sound"),
        (rx, row2, "Machine"),
    ];
    for &(x, y, name) in &panels {
        let p = ui::Panel { x, y, w: pw, h: ph }.draw(frame, PANEL, PANEL_EDGE);
        p.legend(frame, name, PANEL, YELLOW);
    }
    let row = |y: i32, n: i32| y + ui::Panel::TOP_LEGEND + n * ROW;
    let (max, ix, jx) = (pw - ui::Panel::SIDE * 2, lx + ui::Panel::SIDE, rx + ui::Panel::SIDE);
    // Video.
    item(frame, ix, row(row1, 0), max, "VGA 320x200x256", v[VGA] != 0, true);
    item(frame, ix, row(row1, 1), max, "EGA/VGA adapter", v[EGA] != 0, false);
    ui::text_fit(frame, ix, row(row1, 2), "14 lighting shades", GREY, max);
    // Input.
    item(frame, jx, row(row1, 0), max, "Keyboard", true, true);
    item(frame, jx, row(row1, 1), max, "Mouse (unused)", v[MOUSE] != 0, false);
    item(frame, jx, row(row1, 2), max, "Joystick (unused)", v[JOYSTICK] != 0, false);
    // Sound.
    item(frame, ix, row(row2, 0), max, "PC speaker (effects)", true, false);
    item(frame, ix, row(row2, 1), max, "AdLib OPL2 (music)", v[ADLIB] != 0, false);
    item(frame, ix, row(row2, 2), max, "Sound Blaster", v[BLASTER] != 0, false);
    // Machine.
    ui::text_fit(frame, jx, row(row2, 0), "Processor", GREY, 56);
    ui::text_fit(frame, jx + 62, row(row2, 0), cpu, WHITE, max - 62);
    item(frame, jx, row(row2, 1), max, "x87 maths (needed)", v[FPU] != 0, true);
    let mut line = ui::Buf::new();
    let _ = core::fmt::Write::write_fmt(&mut line, format_args!("{} MB memory free", v[MEMORY_KB] / 1024));
    ui::text_fit(frame, jx, row(row2, 2), line.as_str(), WHITE, max);

    // Speed and the footer.
    let fps = v[FPS];
    let (verdict, colour) = if fps == 0 {
        ("", GREY)
    } else if fps >= 90 {
        ("smooth", GREEN)
    } else if fps >= 62 {
        ("fine", GREEN)
    } else if fps >= 40 {
        ("borderline", YELLOW)
    } else {
        ("too slow", RED)
    };
    let foot = ui::Panel { x: 24, y: 152, w: 272, h: 44 }.draw(frame, TITLE_PANEL, MAGENTA);
    let (fx, fw) = (foot.left(), foot.inner_w());
    let y0 = foot.top(false);
    if fps > 0 {
        let mut speed = ui::Buf::new();
        let _ = core::fmt::Write::write_fmt(&mut speed, format_args!("Speed test: {fps} frames a second - "));
        let end = ui::text_fit(frame, fx, y0, speed.as_str(), WHITE, fw);
        ui::text_fit(frame, end, y0, verdict, colour, fx + fw - end);
    }
    let using = match v[SOUND_USING] {
        1 => "AdLib music and effects",
        2 => "PC speaker effects",
        4 => "AdLib music, speaker effects",
        _ => "no sound",
    };
    let mut keys = ui::Buf::new();
    let _ = core::fmt::Write::write_fmt(&mut keys, format_args!("Sound: {using} (A, M, P, O)"));
    ui::text_fit(frame, fx, y0 + 10, keys.as_str(), CYAN, fw);
    ui::text_centred(frame, W as i32 / 2, y0 + 20, "Press Enter to play, Esc to quit", YELLOW);
}
