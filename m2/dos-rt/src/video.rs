// Drawing for DOS: the sim's instance buffer blitted into a 320x200 8-bit frame (VGA mode 13h).
//
// Sprites come from `m3/assets/sprites.bin` (see m3/export-sprites.ts): 0 is transparent, 1..16 the EGA colours.
// The frame uses palette slots 1..16 for the sprites and 17 for the level's clear colour.

use core::sync::atomic::{AtomicU32, Ordering};
use lf_episode_1::state::{LEVEL_ID, PLAYER_DEAD, WON};
use lf_episode_1::{
    enter_level, game_new, instance_ptr, out_ptr, render, set_view, sprite_table_ptr, state_get,
    step, table_len,
};

static SPRITES: &[u8] = include_bytes!("../../../m3/assets/sprites.bin");

pub const W: usize = 320;
pub const H: usize = 200;
/// Pixels to a tile: the sim's world unit.
const TILE: f32 = 16.0;
/// The palette slot the clear colour is written to.
const CLEAR_SLOT: u8 = 17;

/// The EGA colours (sprite pixel 1..16) as 0..255 channels, in the sprite DSL's order.
const EGA: [[u8; 3]; 16] = [
    [0, 0, 0],
    [0, 0, 170],
    [0, 170, 0],
    [0, 170, 170],
    [170, 0, 0],
    [170, 0, 170],
    [170, 85, 0],
    [170, 170, 170],
    [85, 85, 85],
    [85, 85, 255],
    [85, 255, 85],
    [85, 255, 255],
    [255, 85, 85],
    [255, 85, 255],
    [255, 255, 85],
    [255, 255, 255],
];

/// For a tint (0xRRGGBB, white is none) the sprite colour each pixel value 0..16 becomes: the EGA colour nearest
/// to the pixel multiplied by the tint, which is how the web renderer's shader tints.
fn tint_lut(tint: u32) -> [u8; 17] {
    let t = [(tint >> 16) & 255, (tint >> 8) & 255, tint & 255];
    let mut lut = [0u8; 17];
    for (i, c) in EGA.iter().enumerate() {
        let want = [
            u32::from(c[0]) * t[0] / 255,
            u32::from(c[1]) * t[1] / 255,
            u32::from(c[2]) * t[2] / 255,
        ];
        let mut best = (u32::MAX, 0usize);
        for (j, e) in EGA.iter().enumerate() {
            let d: u32 = (0..3)
                .map(|k| {
                    let x = want[k] as i32 - i32::from(e[k]);
                    (x * x) as u32
                })
                .sum();
            if d < best.0 {
                best = (d, j);
            }
        }
        lut[i + 1] = best.1 as u8 + 1;
    }
    lut
}

fn u16_at(o: usize) -> usize {
    usize::from(SPRITES[o]) | usize::from(SPRITES[o + 1]) << 8
}

fn u32_at(o: usize) -> usize {
    u16_at(o) | u16_at(o + 2) << 16
}

/// One sprite of the sheet.
#[derive(Clone, Copy)]
struct Sprite {
    w: usize,
    h: usize,
    /// Every pixel is drawn (a tile): rows copy whole.
    opaque: bool,
    /// Every pixel is this colour, when the sprite is one flat colour (a fill): a row is one `fill`.
    flat: Option<u8>,
    pix: &'static [u8],
}

fn sprite(i: usize) -> Option<Sprite> {
    if i >= u16_at(0) {
        return None;
    }
    let o = u32_at(2 + i * 4);
    let (w, h) = (u16_at(o), u16_at(o + 2));
    let flags = SPRITES[o + 4];
    Some(Sprite {
        w,
        h,
        opaque: flags & 1 != 0,
        flat: (flags & 2 != 0).then(|| SPRITES[o + 5]),
        pix: &SPRITES[o + 6..o + 6 + w * h],
    })
}

/// Starts a level: a new game, the level, the 20x12.5 tile view of a 320x200 screen, and the sprite sizes the
/// sim needs to build its instances (the web shell fills the same table from its atlas).
pub fn start(level: u32) {
    game_new();
    goto(level);
    set_view(f64::from(W as u32) / 32.0, f64::from(H as u32) / 32.0);
    let n = table_len(0) as usize;
    let table = unsafe { core::slice::from_raw_parts_mut(sprite_table_ptr(), n * 6) };
    for (i, s) in table.chunks_exact_mut(6).enumerate() {
        let (w, h) = sprite(i).map_or((1, 1), |sp| (sp.w, sp.h));
        s.copy_from_slice(&[i as f32, 0.0, 1.0, 1.0, w as f32, h as f32]);
    }
}

/// The number of levels in the sim's table.
const LEVELS: u32 = 16;
/// Ticks Ben has been dead, and ticks since the level was won: the demo restarts or moves on after a pause.
static DEAD_FOR: AtomicU32 = AtomicU32::new(0);
static WON_FOR: AtomicU32 = AtomicU32::new(0);

/// Enters `level` (wrapping round the table).
pub fn goto(level: u32) {
    enter_level(level % LEVELS);
    DEAD_FOR.store(0, Ordering::Relaxed);
    WON_FOR.store(0, Ordering::Relaxed);
}

pub fn level() -> u32 {
    state_get(LEVEL_ID) as u32
}

/// A scripted walk to the right with a jump now and then.
pub fn scripted_input(tick: u32) -> u32 {
    const RIGHT: u32 = 2;
    const JUMP: u32 = 16;
    RIGHT | if tick % 45 < 14 { JUMP } else { 0 }
}

/// One tick with the given input. The demo has no game flow of its own, so a death restarts the level after a
/// pause and a win moves on to the next.
pub fn advance(held: u32) {
    step(held);
    let dead = if state_get(PLAYER_DEAD) != 0.0 { DEAD_FOR.load(Ordering::Relaxed) + 1 } else { 0 };
    DEAD_FOR.store(dead, Ordering::Relaxed);
    let won = if state_get(WON) != 0.0 { WON_FOR.load(Ordering::Relaxed) + 1 } else { 0 };
    WON_FOR.store(won, Ordering::Relaxed);
    if dead >= 100 {
        goto(level());
    } else if won >= 150 {
        goto(level() + 1);
    }
}

/// Draws the current scene into `frame` (W*H bytes). Returns the clear colour as 0x00RRGGBB with 6-bit channels,
/// for palette slot 17.
pub fn draw(frame: &mut [u8], alpha: f32) -> u32 {
    let count = build(alpha);
    paint(frame, count)
}

/// The sim writes the scene into its instance buffer; returns how many sprites.
pub fn build(alpha: f32) -> usize {
    render(f64::from(alpha), lf_episode_1::render::flags::CULLING) as usize
}

/// Draws what `build` wrote into `frame`.
pub fn paint(frame: &mut [u8], count: usize) -> u32 {
    let out = unsafe { core::slice::from_raw_parts(out_ptr(), 32) };
    let (cam_x, cam_y) = (out[1], out[2]);
    let six = |v: f32| (v.clamp(0.0, 1.0) * 63.0 + 0.5) as u32;
    let clear = six(out[5]) << 16 | six(out[6]) << 8 | six(out[7]);
    // The list is in painter's order and little of the screen is covered twice, so it is drawn back to front.
    frame.fill(CLEAR_SLOT);
    let inst = unsafe { core::slice::from_raw_parts(instance_ptr(), count * 20) };
    let mut tints = TintCache::new();
    for b in inst.chunks_exact(20) {
        blit(frame, b, cam_x, cam_y, &mut tints);
    }
    clear
}

/// The last few tints seen and their colour tables, so a tint shared by many sprites is worked out once a frame.
struct TintCache {
    keys: [u32; 4],
    luts: [[u8; 17]; 4],
    next: usize,
}

impl TintCache {
    fn new() -> Self {
        TintCache { keys: [0xff_ffff; 4], luts: [[0; 17]; 4], next: 0 }
    }

    fn get(&mut self, tint: u32) -> [u8; 17] {
        for i in 0..4 {
            if self.keys[i] == tint {
                return self.luts[i];
            }
        }
        let lut = tint_lut(tint);
        self.keys[self.next] = tint;
        self.luts[self.next] = lut;
        self.next = (self.next + 1) % 4;
        lut
    }
}

/// `v` rounded to the nearest integer (positions here are above -1000, so no floor call is needed).
fn round(v: f32) -> i32 {
    (v + 1000.5) as i32 - 1000
}

const IDENTITY: [u8; 17] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];

fn blit(frame: &mut [u8], b: &[f32], cam_x: f32, cam_y: f32, tints: &mut TintCache) {
    // Alpha, emissive and actor share one float: alpha + 2 * emissive + 4 * actor.
    let flags = b[19];
    let w2 = if flags >= 3.5 { flags - 4.0 } else { flags };
    let alpha = if w2 >= 1.5 { w2 - 2.0 } else { w2 };
    if alpha < 0.5 {
        return;
    }
    let cx = W as f32 / 2.0 + (b[12] - cam_x) * TILE;
    let cy = H as f32 / 2.0 - (b[13] - cam_y) * TILE;
    let (m00, m01, m10, m11) = (b[0], b[1], b[4], b[5]);
    // Most sprites are not turned: they are copied, flipped or scaled by whole steps of fixed-point, with no
    // floating-point work per pixel (the x87 is slow, and DOSBox's emulation of it slower).
    if m01 == 0.0 && m10 == 0.0 {
        let (dw, dh) = (round(m00.abs() * TILE), round(m11.abs() * TILE));
        if dw <= 0 || dh <= 0 {
            return;
        }
        let x0 = round(cx - m00.abs() * TILE * 0.5);
        let y0 = round(cy - m11.abs() * TILE * 0.5);
        if x0 >= W as i32 || y0 >= H as i32 || x0 + dw <= 0 || y0 + dh <= 0 {
            return;
        }
        let Some(sp) = sprite(b[16] as usize) else {
            return;
        };
        let (sw, sh) = (sp.w, sp.h);
        let tint = b[18] as u32;
        let identity = tint == 0xff_ffff;
        let lut = if identity { IDENTITY } else { tints.get(tint) };
        let (flip_x, flip_y) = (m00 < 0.0, m11 < 0.0);
        let (cx0, cx1) = (x0.max(0), (x0 + dw).min(W as i32));
        let (cy0, cy1) = (y0.max(0), (y0 + dh).min(H as i32));
        let span = (cx1 - cx0) as usize;
        // A flat sprite (the big background fills) is one `fill` a row.
        if let Some(c) = sp.flat {
            let colour = lut[usize::from(c)];
            for y in cy0..cy1 {
                frame[y as usize * W + cx0 as usize..][..span].fill(colour);
            }
            return;
        }
        if dw as usize == sw && dh as usize == sh {
            // Drawn at its own size, the common case: rows are copied, whole when the sprite is opaque.
            let skip = (cx0 - x0) as usize;
            for y in cy0..cy1 {
                let mut r = (y - y0) as usize;
                if flip_y {
                    r = sh - 1 - r;
                }
                let src = &sp.pix[r * sw..][..sw];
                let dst = &mut frame[y as usize * W + cx0 as usize..][..span];
                if !flip_x {
                    let s = &src[skip..][..span];
                    if sp.opaque && identity {
                        dst.copy_from_slice(s);
                    } else if sp.opaque {
                        for (d, &c) in dst.iter_mut().zip(s) {
                            *d = lut[usize::from(c)];
                        }
                    } else {
                        for (d, &c) in dst.iter_mut().zip(s) {
                            if c != 0 {
                                *d = lut[usize::from(c)];
                            }
                        }
                    }
                } else {
                    for (i, d) in dst.iter_mut().enumerate() {
                        let c = src[sw - 1 - (skip + i)];
                        if c != 0 {
                            *d = lut[usize::from(c)];
                        }
                    }
                }
            }
            return;
        }
        // Scaled: fixed-point steps through the sprite, no floating point per pixel.
        let ustep = ((sw as i32) << 16) / dw;
        let vstep = ((sh as i32) << 16) / dh;
        for y in cy0..cy1 {
            let mut row = (((y - y0) * vstep + vstep / 2) >> 16) as usize;
            if flip_y {
                row = sh - 1 - row.min(sh - 1);
            }
            let src = &sp.pix[row.min(sh - 1) * sw..][..sw];
            let dst = &mut frame[y as usize * W..][..W];
            let mut u = (cx0 - x0) * ustep + ustep / 2;
            for x in cx0..cx1 {
                let mut col = (u >> 16) as usize;
                if flip_x {
                    col = sw - 1 - col.min(sw - 1);
                }
                let c = src[col.min(sw - 1)];
                if c != 0 {
                    dst[x as usize] = lut[usize::from(c)];
                }
                u += ustep;
            }
        }
        return;
    }
    blit_turned(frame, b, cx, cy, tints);
}

/// A sprite that is rotated (a few effects): each screen pixel is mapped back through the instance's matrix.
fn blit_turned(frame: &mut [u8], b: &[f32], cx: f32, cy: f32, tints: &mut TintCache) {
    let Some(sp) = sprite(b[16] as usize) else {
        return;
    };
    let (sw, sh, pix) = (sp.w, sp.h, sp.pix);
    let tint = b[18] as u32;
    let lut = if tint == 0xff_ffff { IDENTITY } else { tints.get(tint) };
    let (m00, m01, m10, m11) = (b[0], b[1], b[4], b[5]);
    let det = m00 * m11 - m10 * m01;
    if det == 0.0 {
        return;
    }
    let ex = (m00.abs() + m10.abs()) * 0.5 * TILE;
    let ey = (m01.abs() + m11.abs()) * 0.5 * TILE;
    let x0 = ((cx - ex) as i32).max(0);
    let x1 = ((cx + ex + 1.0) as i32).min(W as i32);
    let y0 = ((cy - ey) as i32).max(0);
    let y1 = ((cy + ey + 1.0) as i32).min(H as i32);
    for y in y0..y1 {
        for x in x0..x1 {
            let wx = (x as f32 + 0.5 - cx) / TILE;
            let wy = -(y as f32 + 0.5 - cy) / TILE;
            let px = (m11 * wx - m10 * wy) / det;
            let py = (-m01 * wx + m00 * wy) / det;
            if !(-0.5..0.5).contains(&px) || !(-0.5..0.5).contains(&py) {
                continue;
            }
            let col = ((px + 0.5) * sw as f32) as usize;
            let row = ((0.5 - py) * sh as f32) as usize;
            let c = pix[row.min(sh - 1) * sw + col.min(sw - 1)];
            if c != 0 {
                frame[y as usize * W + x as usize] = lut[usize::from(c)];
            }
        }
    }
}
