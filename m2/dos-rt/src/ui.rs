// Text and boxes drawn over the finished frame in full-brightness palette colours: the Fizz font, panels and word wrap.
//
// A frame byte here is `level * 16 + colour`; the user interface is drawn after the lighting pass, so it always uses level
// 14 (full brightness): `FULL + colour`, with the colour an EGA index 0..15.

use crate::ui_data::{Glyph, FONT};
use crate::video::{H, W};

/// The palette base of full-brightness colours.
pub const FULL: u8 = 14 * 16;

/// EGA colour indices.
pub const BLACK: u8 = 0;
pub const GREY: u8 = 7;
pub const DARK: u8 = 8;
pub const GREEN: u8 = 10;
pub const CYAN: u8 = 11;
pub const RED: u8 = 12;
pub const YELLOW: u8 = 14;
pub const WHITE: u8 = 15;
pub const BLUE: u8 = 9;

/// Pixels a line of text takes: the capitals are seven rows, descenders two more, and one row of air.
pub const LINE: i32 = 10;
/// The width of a space.
const SPACE: i32 = 3;

fn glyph(c: char) -> Option<&'static Glyph> {
    FONT.binary_search_by_key(&(c as u32), |g| g.cp).ok().map(|i| &FONT[i])
}

/// The width of `s` in pixels: each glyph and a pixel after it.
pub fn text_width(s: &str) -> i32 {
    s.chars().map(advance).sum()
}

fn advance(c: char) -> i32 {
    if c == ' ' {
        SPACE
    } else {
        glyph(c).map_or(SPACE, |g| i32::from(g.w) + 1)
    }
}

fn put(frame: &mut [u8], x: i32, y: i32, colour: u8) {
    if x >= 0 && y >= 0 && (x as usize) < W && (y as usize) < H {
        frame[y as usize * W + x as usize] = FULL + colour;
    }
}

fn draw_glyphs(frame: &mut [u8], x: i32, y: i32, s: &str, colour: u8) -> i32 {
    let mut at = x;
    for c in s.chars() {
        if let Some(g) = glyph(c) {
            for (row, bits) in g.rows.iter().enumerate() {
                for col in 0..g.w {
                    if bits >> (g.w - 1 - col) & 1 != 0 {
                        put(frame, at + i32::from(col), y + row as i32, colour);
                    }
                }
            }
        }
        at += advance(c);
    }
    at
}

/// Draws `s` with its top at `y`, with a black shadow so it reads over anything. Returns where the text ended.
pub fn text(frame: &mut [u8], x: i32, y: i32, s: &str, colour: u8) -> i32 {
    draw_glyphs(frame, x + 1, y + 1, s, BLACK);
    draw_glyphs(frame, x, y, s, colour)
}

/// Draws `s` centred on `cx`.
pub fn text_centred(frame: &mut [u8], cx: i32, y: i32, s: &str, colour: u8) {
    text(frame, cx - text_width(s) / 2, y, s, colour);
}

pub fn fill(frame: &mut [u8], x: i32, y: i32, w: i32, h: i32, colour: u8) {
    for yy in y.max(0)..(y + h).min(H as i32) {
        for xx in x.max(0)..(x + w).min(W as i32) {
            frame[yy as usize * W + xx as usize] = FULL + colour;
        }
    }
}

pub fn outline(frame: &mut [u8], x: i32, y: i32, w: i32, h: i32, colour: u8) {
    fill(frame, x, y, w, 1, colour);
    fill(frame, x, y + h - 1, w, 1, colour);
    fill(frame, x, y, 1, h, colour);
    fill(frame, x + w - 1, y, 1, h, colour);
}

/// Darkens what is under a rectangle by `steps` brightness levels, so text on it reads while the scene still shows through.
pub fn dim(frame: &mut [u8], x: i32, y: i32, w: i32, h: i32, steps: u8) {
    for yy in y.max(0)..(y + h).min(H as i32) {
        for xx in x.max(0)..(x + w).min(W as i32) {
            let p = &mut frame[yy as usize * W + xx as usize];
            *p = if *p == 255 {
                16
            } else {
                ((*p >> 4).saturating_sub(steps)).max(1) << 4 | (*p & 15)
            };
        }
    }
}

/// How many lines `s` takes in `width` pixels, filling `lines` with where each begins and ends (byte offsets).
pub fn wrap(s: &str, width: i32, lines: &mut [(usize, usize)]) -> usize {
    let mut n = 0;
    let mut start = 0;
    let mut line_width = 0;
    let mut last_break: Option<usize> = None;
    let mut i = 0;
    let bytes = s.as_bytes();
    while i < bytes.len() && n < lines.len() {
        let c = s[i..].chars().next().unwrap_or(' ');
        let w = advance(c);
        if c == ' ' {
            last_break = Some(i);
        }
        if line_width + w > width && i > start {
            let end = last_break.filter(|&b| b > start).unwrap_or(i);
            lines[n] = (start, end);
            n += 1;
            start = if s[end..].starts_with(' ') { end + 1 } else { end };
            i = start;
            line_width = 0;
            last_break = None;
            continue;
        }
        line_width += w;
        i += c.len_utf8();
    }
    if start < bytes.len() && n < lines.len() {
        lines[n] = (start, bytes.len());
        n += 1;
    }
    n
}

/// Draws `s` word-wrapped in `width` pixels; returns the lines drawn.
pub fn paragraph(frame: &mut [u8], x: i32, y: i32, width: i32, s: &str, colour: u8) -> i32 {
    let mut lines = [(0usize, 0usize); 12];
    let n = wrap(s, width, &mut lines);
    for (i, &(a, b)) in lines[..n].iter().enumerate() {
        text(frame, x, y + i as i32 * LINE, &s[a..b], colour);
    }
    n as i32
}

/// A small string builder on the stack, so the shell can put numbers in sentences without an allocator.
pub struct Buf {
    bytes: [u8; 400],
    len: usize,
}

impl Buf {
    pub fn new() -> Self {
        Buf { bytes: [0; 400], len: 0 }
    }
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes[..self.len]).unwrap_or("")
    }
}

impl core::fmt::Write for Buf {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let room = self.bytes.len() - self.len;
        let take = s.len().min(room);
        self.bytes[self.len..self.len + take].copy_from_slice(&s.as_bytes()[..take]);
        self.len += take;
        Ok(())
    }
}
