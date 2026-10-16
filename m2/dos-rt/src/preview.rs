// The start-up screen: what the machine has and whether the demo will run well on it, in the manner of Keen's own.
//
// The C side finds out about the hardware (video, input, sound, memory, the speed of a few trial frames) and hands over a
// list of numbers; this draws them.

use crate::ui::{self, BLACK, CYAN, DARK, GREEN, GREY, RED, WHITE, YELLOW};
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
const MAGENTA: u8 = 13;

/// A panel with a heading: filled, with a light double edge and a black shadow, as Keen's are.
fn panel(frame: &mut [u8], x: i32, y: i32, w: i32, h: i32, title: &str) {
    ui::fill(frame, x + 3, y + 3, w, h, BLACK);
    ui::fill(frame, x, y, w, h, PANEL);
    ui::outline(frame, x + 2, y + 2, w - 4, h - 4, PANEL_EDGE);
    ui::outline(frame, x + 4, y + 4, w - 8, h - 8, PANEL_EDGE);
    ui::text(frame, x + 10, y + 6, title, YELLOW);
}

/// A line with a tick or a cross: wanted and found is white, not found is dim.
fn item(frame: &mut [u8], x: i32, y: i32, label: &str, found: bool, needed: bool) {
    if found {
        ui::tick_mark(frame, x, y, GREEN);
        ui::text(frame, x + 11, y, label, WHITE);
    } else {
        ui::cross_mark(frame, x, y, if needed { RED } else { DARK });
        ui::text(frame, x + 11, y, label, if needed { RED } else { DARK });
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
    // The title.
    let (tx, ty, tw, th) = (24, 4, 272, 40);
    ui::fill(frame, tx + 3, ty + 3, tw, th, BLACK);
    ui::fill(frame, tx, ty, tw, th, 5);
    ui::outline(frame, tx + 2, ty + 2, tw - 4, th - 4, MAGENTA);
    let w = ui::text_width("Lieutenant Fizz") * 2;
    ui::text_big(frame, (W as i32 - w) / 2, ty + 6, "Lieutenant Fizz", WHITE, 2);
    ui::text_centred(frame, W as i32 / 2, ty + 22, "Episode 1 - The Cocoa Caper", YELLOW);
    ui::text_centred(frame, W as i32 / 2, ty + 30, "A Liminal HQ DOS port - work in progress", CYAN);

    let (lx, rx, w2, h2) = (8, 164, 148, 48);
    let (row1, row2) = (50, 102);
    // Video.
    panel(frame, lx, row1, w2, h2, "Video");
    item(frame, lx + 10, row1 + 16, "VGA 320x200x256", v[VGA] != 0, true);
    item(frame, lx + 10, row1 + 26, "EGA/VGA adapter", v[EGA] != 0, false);
    ui::text(frame, lx + 10, row1 + 36, "14 lighting shades", GREY);
    // Input.
    panel(frame, rx, row1, w2, h2, "Input");
    item(frame, rx + 10, row1 + 16, "Keyboard", true, true);
    item(frame, rx + 10, row1 + 26, "Mouse (unused)", v[MOUSE] != 0, false);
    item(frame, rx + 10, row1 + 36, "Joystick (unused)", v[JOYSTICK] != 0, false);
    // Sound.
    panel(frame, lx, row2, w2, h2, "Sound");
    item(frame, lx + 10, row2 + 16, "PC speaker (effects)", true, false);
    item(frame, lx + 10, row2 + 26, "AdLib OPL2 (music)", v[ADLIB] != 0, false);
    item(frame, lx + 10, row2 + 36, "Sound Blaster", v[BLASTER] != 0, false);
    // Machine.
    panel(frame, rx, row2, w2, h2, "Machine");
    ui::text(frame, rx + 10, row2 + 16, "Processor", GREY);
    ui::text(frame, rx + 72, row2 + 16, cpu, WHITE);
    item(frame, rx + 10, row2 + 26, "x87 maths (needed)", v[FPU] != 0, true);
    let mut line = ui::Buf::new();
    let mb = v[MEMORY_KB] / 1024;
    let _ = core::fmt::Write::write_fmt(&mut line, format_args!("{mb} MB memory free"));
    ui::text(frame, rx + 10, row2 + 36, line.as_str(), WHITE);

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
        ("too slow, give it more cycles", RED)
    };
    let (fx, fy, fw, fh) = (24, 154, 272, 42);
    ui::fill(frame, fx + 3, fy + 3, fw, fh, BLACK);
    ui::fill(frame, fx, fy, fw, fh, 5);
    ui::outline(frame, fx + 2, fy + 2, fw - 4, fh - 4, MAGENTA);
    if fps > 0 {
        let mut speed = ui::Buf::new();
        let _ = core::fmt::Write::write_fmt(&mut speed, format_args!("Speed test: {fps} frames a second - "));
        let end = ui::text(frame, fx + 10, fy + 7, speed.as_str(), WHITE);
        ui::text(frame, end, fy + 7, verdict, colour);
    }
    let using = match v[SOUND_USING] {
        1 => "AdLib music and effects",
        2 => "PC speaker effects",
        _ => "no sound",
    };
    let mut keys = ui::Buf::new();
    let _ = core::fmt::Write::write_fmt(&mut keys, format_args!("Sound: {using} (A, P, O change it)"));
    ui::text(frame, fx + 10, fy + 17, keys.as_str(), CYAN);
    ui::text_centred(frame, W as i32 / 2, fy + 28, "Press Enter to play, Esc to quit", YELLOW);
}
