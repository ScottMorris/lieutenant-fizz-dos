// Text and boxes drawn over the finished frame in full-brightness palette colours: the Fizz font, panels and word wrap.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT
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
        if c == ' ' {
            at += SPACE;
            continue;
        }
        let Some(g) = glyph(c) else {
            at += SPACE;
            continue;
        };
        let w = i32::from(g.w);
        if at >= 0 && y >= 0 && at + w <= W as i32 && y + g.rows.len() as i32 <= H as i32 {
            // Wholly on the screen, the usual case: no clipping a pixel at a time, and the set bits are walked from the right.
            for (row, &bits) in g.rows.iter().enumerate() {
                let line = &mut frame[(y as usize + row) * W + at as usize..][..g.w as usize];
                let mut bits = u32::from(bits);
                while bits != 0 {
                    let k = bits.trailing_zeros() as usize;
                    line[g.w as usize - 1 - k] = FULL + colour;
                    bits &= bits - 1;
                }
            }
        } else {
            for (row, bits) in g.rows.iter().enumerate() {
                for col in 0..g.w {
                    if bits >> (g.w - 1 - col) & 1 != 0 {
                        put(frame, at + i32::from(col), y + row as i32, colour);
                    }
                }
            }
        }
        at += w + 1;
    }
    at
}

/// Draws `s` with its top at `y`, with a black shadow so it reads over anything. Returns where the text ended.
pub fn text(frame: &mut [u8], x: i32, y: i32, s: &str, colour: u8) -> i32 {
    draw_glyphs(frame, x + 1, y + 1, s, BLACK);
    draw_glyphs(frame, x, y, s, colour)
}

/// Draws `s` with every pixel `k` pixels square (a headline), with a shadow.
pub fn text_big(frame: &mut [u8], x: i32, y: i32, s: &str, colour: u8, k: i32) -> i32 {
    let mut end = x;
    for pass in 0..2 {
        let (dx, dy, c) = if pass == 0 { (k, k, BLACK) } else { (0, 0, colour) };
        let mut at = x + dx;
        for ch in s.chars() {
            if let Some(g) = glyph(ch) {
                for (row, bits) in g.rows.iter().enumerate() {
                    for col in 0..g.w {
                        if bits >> (g.w - 1 - col) & 1 != 0 {
                            fill(frame, at + i32::from(col) * k, y + dy + row as i32 * k, k, k, c);
                        }
                    }
                }
            }
            at += advance(ch) * k;
        }
        end = at - dx;
    }
    end
}

/// The logo's lettering: `colour` letters with a coloured edge one glyph pixel under them and a black line under that, as the web
/// build's wordmark has. Every pixel is `k` pixels square. Returns where the text ended.
pub fn text_logo(frame: &mut [u8], x: i32, y: i32, s: &str, colour: u8, edge: u8, k: i32) -> i32 {
    let mut end = x;
    for (dy, c) in [(2 * k, BLACK), (k, edge), (0, colour)] {
        let mut at = x;
        for ch in s.chars() {
            if let Some(g) = glyph(ch) {
                for (row, bits) in g.rows.iter().enumerate() {
                    for col in 0..g.w {
                        if bits >> (g.w - 1 - col) & 1 != 0 {
                            fill(frame, at + i32::from(col) * k, y + dy + row as i32 * k, k, k, c);
                        }
                    }
                }
            }
            at += advance(ch) * k;
        }
        end = at;
    }
    end
}

/// A tick mark, 7 pixels wide.
pub fn tick_mark(frame: &mut [u8], x: i32, y: i32, colour: u8) {
    for (dx, dy) in [(0, 3), (1, 4), (2, 5), (3, 4), (4, 3), (5, 2), (6, 1), (7, 0)] {
        put(frame, x + dx, y + dy, colour);
        put(frame, x + dx, y + dy + 1, colour);
    }
}

/// A cross, 6 pixels wide.
pub fn cross_mark(frame: &mut [u8], x: i32, y: i32, colour: u8) {
    for d in 0..6 {
        put(frame, x + d, y + 1 + d, colour);
        put(frame, x + 5 - d, y + 1 + d, colour);
    }
}

/// Draws `s` but no wider than `max` pixels: a label that would run on is cut short with two dots, so text never reaches a border.
pub fn text_fit(frame: &mut [u8], x: i32, y: i32, s: &str, colour: u8, max: i32) -> i32 {
    if text_width(s) <= max {
        return text(frame, x, y, s, colour);
    }
    let mut end = 0;
    let mut w = text_width("..");
    for (i, c) in s.char_indices() {
        w += advance(c);
        if w > max {
            break;
        }
        end = i + c.len_utf8();
    }
    let mut cut = Buf::new();
    let _ = core::fmt::Write::write_str(&mut cut, &s[..end]);
    let _ = core::fmt::Write::write_str(&mut cut, "..");
    text(frame, x, y, cut.as_str(), colour)
}

/// A panel with a black shadow, a fill and a double edge drawn inside it. The edge takes the outer four pixels, so
/// text belongs in `Panel::content`, which keeps clear of it.
#[derive(Clone, Copy)]
pub struct Panel {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Panel {
    /// Pixels between the outer edge and where text may start on the sides and the bottom, and at the top.
    pub const SIDE: i32 = 9;
    pub const BOTTOM: i32 = 7;
    pub const TOP: i32 = 8;
    /// The top when a legend sits on the top edge.
    pub const TOP_LEGEND: i32 = 12;

    pub fn draw(self, frame: &mut [u8], fill_colour: u8, edge: u8) -> Self {
        fill(frame, self.x + 3, self.y + 3, self.w, self.h, BLACK);
        fill(frame, self.x, self.y, self.w, self.h, fill_colour);
        outline(frame, self.x + 1, self.y + 1, self.w - 2, self.h - 2, edge);
        outline(frame, self.x + 3, self.y + 3, self.w - 6, self.h - 6, edge);
        self
    }

    /// Text on the top edge, with the panel's colour behind it so the edge does not run through the letters.
    pub fn legend(self, frame: &mut [u8], s: &str, fill_colour: u8, colour: u8) {
        let w = text_width(s) + 6;
        fill(frame, self.x + Self::SIDE - 3, self.y, w, 9, fill_colour);
        text(frame, self.x + Self::SIDE, self.y, s, colour);
    }

    /// Where the first line of text goes, and how wide a line may be.
    pub fn left(self) -> i32 {
        self.x + Self::SIDE
    }
    pub fn inner_w(self) -> i32 {
        self.w - Self::SIDE * 2
    }
    pub fn top(self, legend: bool) -> i32 {
        self.y + if legend { Self::TOP_LEGEND } else { Self::TOP }
    }
}

/// Draws `s` in `colour` with no shadow, for dark ink on a bright plate.
pub fn text_ink(frame: &mut [u8], x: i32, y: i32, s: &str, colour: u8) -> i32 {
    draw_glyphs(frame, x, y, s, colour)
}

/// Draws the first `upto` characters of `s`, word-wrapped as a whole (so a word does not jump line as it is typed).
pub fn paragraph_upto(frame: &mut [u8], x: i32, y: i32, width: i32, s: &str, colour: u8, upto: usize) {
    let mut lines = [(0usize, 0usize); 12];
    let n = wrap(s, width, &mut lines);
    let mut left = upto;
    for (i, &(a, b)) in lines[..n].iter().enumerate() {
        let line = &s[a..b];
        let count = line.chars().count();
        if left == 0 {
            break;
        }
        let shown = if left >= count { line } else { &line[..line.char_indices().nth(left).map_or(line.len(), |(i, _)| i)] };
        text(frame, x, y + i as i32 * LINE, shown, colour);
        left = left.saturating_sub(count + 1);
    }
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
    let (x0, x1) = (x.max(0) as usize, (x + w).min(W as i32).max(0) as usize);
    let (y0, y1) = (y.max(0) as usize, (y + h).min(H as i32).max(0) as usize);
    if x1 <= x0 || y1 <= y0 {
        return;
    }
    // One lookup a pixel for the odd ends and for words that hold the clear colour; four pixels at a time for the rest.
    let mut lut = [0u8; 256];
    for (p, o) in lut.iter_mut().enumerate() {
        *o = if p == 255 { 16 } else { ((p as u8 >> 4).saturating_sub(steps)).max(1) << 4 | (p as u8 & 15) };
    }
    // For each byte's level h, `(h | 0x80) - (steps + 1)` keeps its top bit when h > steps, and its low bits are then h - steps - 1;
    // the new level is that plus one, or 1 when the top bit went. No byte borrows from the next.
    let less = u32::from(steps.min(15) + 1) * 0x0101_0101;
    for row in y0..y1 {
        let line = &mut frame[row * W + x0..row * W + x1];
        let n = line.len() / 4;
        let base = line.as_mut_ptr() as *mut u32;
        for k in 0..n {
            // SAFETY: word k is inside the row; x86 reads and writes unaligned words freely.
            let w = unsafe { base.add(k).read_unaligned() };
            let out = if w == 0xffff_ffff {
                0x1010_1010
            } else if (!w).wrapping_sub(0x0101_0101) & w & 0x8080_8080 != 0 {
                // A byte is 255, the clear colour, which dims to black (slot 16).
                u32::from_le_bytes(w.to_le_bytes().map(|p| lut[usize::from(p)]))
            } else {
                let t = (((w >> 4) & 0x0f0f_0f0f) | 0x8080_8080) - less;
                let top = t & 0x8080_8080;
                let keep = top | (top - (top >> 7));
                (((t & keep & 0x7f7f_7f7f) + 0x0101_0101) << 4) | (w & 0x0f0f_0f0f)
            };
            unsafe { base.add(k).write_unaligned(out) };
        }
        for p in &mut line[n * 4..] {
            *p = lut[usize::from(*p)];
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
