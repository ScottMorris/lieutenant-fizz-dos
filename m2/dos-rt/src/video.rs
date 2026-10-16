// Drawing for DOS: the sim's instance buffer blitted into a 320x200 8-bit frame (VGA mode 13h).
//
// Sprites come from `m3/assets/sprites.bin` (see m3/export-sprites.ts): the EGA colour 0..15, or 255 for transparent.
//
// Lighting by palette. A frame byte is `shade * 16 + colour`. While sprites are drawn the high nibble is only a
// marker for how the pixel is lit: 0 lit, 0xD actor (lit, but never darker than a floor), 0xE glowing (full
// brightness). A last pass swaps the marker for the brightness level of the 8x8-pixel cell the pixel is in, which
// comes from the sim's ambient light and point lights. The palette holds every colour at 14 brightness levels.

use core::sync::atomic::{AtomicU32, Ordering};
use lf_episode_1::state::{BOSS_HP, LEVEL_ID};
use lf_episode_1::{
    enter_level, enter_map, game_new, instance_ptr, lights_col_ptr, lights_pos_ptr, out_ptr, render, set_view,
    sprite_table_ptr, state_get, table_len,
};
use lf_episode_1::mode;

static SPRITES: &[u8] = include_bytes!("../../../m3/assets/sprites.bin");

pub const W: usize = 320;
pub const H: usize = 200;
/// Pixels to a tile: the sim's world unit.
const TILE: f32 = 16.0;
/// The frame byte (and palette slot) of the level's clear colour.
const CLEAR_SLOT: u8 = 255;
/// Markers in a frame byte's high nibble before the lighting pass.
const MARK_ACTOR: u8 = 0xD0;
const MARK_GLOW: u8 = 0xE0;
/// Transparent in the sprite sheet.
const CLEAR_PIX: u8 = 255;
/// Brightness levels: 1 (black) to 14 (full), `(level - 1) / 13` of the colour.
const LEVELS: usize = 14;
/// Characters never go darker than this level (about 0.62 of full, as in the web shader).
const ACTOR_FLOOR: u8 = 9;
/// Lighting cells are this many pixels square.
const CELL: usize = 8;
const CELLS_X: usize = W / CELL;
const CELLS_Y: usize = H / CELL;

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

/// For a tint (0xRRGGBB, white is none) the colour each sprite colour becomes: the EGA colour nearest to the pixel
/// multiplied by the tint, which is how the web renderer's shader tints.
fn tint_lut(tint: u32) -> [u8; 16] {
    let t = [(tint >> 16) & 255, (tint >> 8) & 255, tint & 255];
    let mut lut = [0u8; 16];
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
        lut[i] = best.1 as u8;
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
    if level >= MAP {
        goto_map();
    } else {
        goto(level);
    }
    set_view(f64::from(W as u32) / 32.0, f64::from(H as u32) / 32.0);
    let n = table_len(0) as usize;
    let table = unsafe { core::slice::from_raw_parts_mut(sprite_table_ptr(), n * 6) };
    for (i, s) in table.chunks_exact_mut(6).enumerate() {
        let (w, h) = sprite(i).map_or((1, 1), |sp| (sp.w, sp.h));
        s.copy_from_slice(&[i as f32, 0.0, 1.0, 1.0, w as f32, h as f32]);
    }
}

/// The number of levels in the sim's table.
const TABLE_LEVELS: u32 = 16;
/// The level number the demo asks for to mean the overworld.
pub const MAP: u32 = 255;

/// A check: 1 steers the scripted walk to the Crater Fields marker and presses Jump there; 2 steers but never presses.
static PRESS: AtomicU32 = AtomicU32::new(0);

pub fn set_press(mode: i32) {
    PRESS.store(mode as u32, Ordering::Relaxed);
}

/// The render flags the player chose (night).
static NIGHT: AtomicU32 = AtomicU32::new(0);

pub fn set_night(on: bool) {
    NIGHT.store(u32::from(on), Ordering::Relaxed);
}

/// Enters `level` (wrapping round the table).
pub fn goto(level: u32) {
    crate::shell::reset();
    enter_level(level % TABLE_LEVELS);
}

/// Enters the overworld.
pub fn goto_map() {
    crate::shell::reset();
    enter_map();
}

/// The sim's mode: 0 none, 1 attract, 2 map, 3 level.
pub fn sim_mode() -> u32 {
    mode()
}

pub fn boss_hp() -> u32 {
    state_get(BOSS_HP) as u32
}

pub fn level() -> u32 {
    state_get(LEVEL_ID) as u32
}

/// A scripted walk to the right with a jump now and then.
pub fn scripted_input(tick: u32) -> u32 {
    const RIGHT: u32 = 2;
    const UP: u32 = 4;
    const JUMP: u32 = 16;
    if mode() == 2 {
        if PRESS.load(Ordering::Relaxed) != 0 {
            // Steer to the Crater Fields marker (14, 6) and press Jump on it.
            const LEFT: u32 = 1;
            const DOWN: u32 = 8;
            let (x, y) = (state_get(lf_episode_1::state::PLAYER_X), state_get(lf_episode_1::state::PLAYER_Y));
            let (dx, dy) = (14.0 - x, 6.0 - y);
            if dx.abs() < 0.3 && dy.abs() < 0.3 {
                return if PRESS.load(Ordering::Relaxed) == 1 && tick % 6 < 2 { JUMP } else { 0 };
            }
            let mut held = 0;
            if dx > 0.25 { held |= RIGHT } else if dx < -0.25 { held |= LEFT }
            if dy > 0.25 { held |= UP } else if dy < -0.25 { held |= DOWN }
            return held;
        }
        // On the map: east, then north, never pressing a button that would enter a level.
        return if tick % 140 < 80 { RIGHT } else { UP };
    }
    RIGHT | if tick % 45 < 14 { JUMP } else { 0 }
}

/// One tick with the given input: the shell steps the sim and reads its events.
pub fn advance(held: u32) {
    crate::shell::advance(held);
}

/// Draws the current scene into `frame` (W*H bytes). Returns the clear colour as 0x00RRGGBB with 6-bit channels,
/// for palette slot 17.
pub fn draw(frame: &mut [u8], alpha: f32) -> u32 {
    let count = build(alpha);
    paint(frame, count)
}

/// The sim writes the scene into its instance buffer; returns how many sprites.
pub fn build(alpha: f32) -> usize {
    let night = if NIGHT.load(Ordering::Relaxed) != 0 { lf_episode_1::render::flags::NIGHT } else { 0 };
    render(f64::from(alpha), lf_episode_1::render::flags::CULLING | night) as usize
}

/// Draws what `build` wrote into `frame`.
pub fn paint(frame: &mut [u8], count: usize) -> u32 {
    let out = unsafe { core::slice::from_raw_parts(out_ptr(), 32) };
    // The camera is snapped to whole pixels first: a sprite's place is its own world position rounded, less the camera, so
    // things that are still stay still relative to each other as the camera glides.
    let cam_px = (round(out[1] * TILE), round(out[2] * TILE));
    let six = |v: f32| (v.clamp(0.0, 1.0) * 63.0 + 0.5) as u32;
    let clear = six(out[5]) << 16 | six(out[6]) << 8 | six(out[7]);
    let levels = light_levels(out, cam_px);
    // The list is in painter's order and little of the screen is covered twice, so it is drawn back to front.
    frame.fill(CLEAR_SLOT);
    let inst = unsafe { core::slice::from_raw_parts(instance_ptr(), count * 20) };
    let mut tints = TintCache::new();
    for b in inst.chunks_exact(20) {
        blit(frame, b, cam_px, &mut tints);
    }
    shade(frame, &levels);
    crate::shell::overlay(frame, cam_px);
    clear
}

/// The last few tints seen and their colour tables, so a tint shared by many sprites is worked out once a frame.
struct TintCache {
    keys: [u32; 4],
    luts: [[u8; 16]; 4],
    next: usize,
}

impl TintCache {
    fn new() -> Self {
        TintCache { keys: [0xff_ffff; 4], luts: [[0; 16]; 4], next: 0 }
    }

    fn get(&mut self, tint: u32) -> [u8; 16] {
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

const IDENTITY: [u8; 16] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];

fn blit(frame: &mut [u8], b: &[f32], cam_px: (i32, i32), tints: &mut TintCache) {
    // Alpha, emissive and actor share one float: alpha + 2 * emissive + 4 * actor.
    let flags = b[19];
    let w2 = if flags >= 3.5 { flags - 4.0 } else { flags };
    let alpha = if w2 >= 1.5 { w2 - 2.0 } else { w2 };
    if alpha < 0.5 {
        return;
    }
    let mark = if w2 >= 1.5 {
        MARK_GLOW
    } else if flags >= 3.5 {
        MARK_ACTOR
    } else {
        0
    };
    let cx = W as f32 / 2.0 + b[12] * TILE - cam_px.0 as f32;
    let cy = H as f32 / 2.0 - b[13] * TILE + cam_px.1 as f32;
    let (m00, m01, m10, m11) = (b[0], b[1], b[4], b[5]);
    // Most sprites are not turned: they are copied, flipped or scaled by whole steps of fixed-point, with no
    // floating-point work per pixel (the x87 is slow, and DOSBox's emulation of it slower).
    if m01 == 0.0 && m10 == 0.0 {
        let (dw, dh) = (round(m00.abs() * TILE), round(m11.abs() * TILE));
        if dw <= 0 || dh <= 0 {
            return;
        }
        // Rounded in the world first, then moved by the whole-pixel camera.
        let x0 = round(b[12] * TILE - m00.abs() * TILE * 0.5) + W as i32 / 2 - cam_px.0;
        let y0 = H as i32 / 2 + cam_px.1 - round(b[13] * TILE + m11.abs() * TILE * 0.5);
        if x0 >= W as i32 || y0 >= H as i32 || x0 + dw <= 0 || y0 + dh <= 0 {
            return;
        }
        let Some(sp) = sprite(b[16] as usize) else {
            return;
        };
        let (sw, sh) = (sp.w, sp.h);
        let tint = b[18] as u32;
        let identity = tint == 0xff_ffff && mark == 0;
        let mut lut = if tint == 0xff_ffff { IDENTITY } else { tints.get(tint) };
        for l in lut.iter_mut() {
            *l |= mark;
        }
        let (flip_x, flip_y) = (m00 < 0.0, m11 < 0.0);
        let (cx0, cx1) = (x0.max(0), (x0 + dw).min(W as i32));
        let (cy0, cy1) = (y0.max(0), (y0 + dh).min(H as i32));
        let span = (cx1 - cx0) as usize;
        // A flat sprite (the big background fills) is one `fill` a row.
        if let Some(c) = sp.flat {
            let colour = lut[usize::from(c & 15)];
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
                            *d = lut[usize::from(c & 15)];
                        }
                    } else {
                        for (d, &c) in dst.iter_mut().zip(s) {
                            if c != CLEAR_PIX {
                                *d = lut[usize::from(c & 15)];
                            }
                        }
                    }
                } else {
                    for (i, d) in dst.iter_mut().enumerate() {
                        let c = src[sw - 1 - (skip + i)];
                        if c != CLEAR_PIX {
                            *d = lut[usize::from(c & 15)];
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
                if c != CLEAR_PIX {
                    dst[x as usize] = lut[usize::from(c & 15)];
                }
                u += ustep;
            }
        }
        return;
    }
    blit_turned(frame, b, cx, cy, mark, tints);
}

/// A sprite that is rotated (a few effects): each screen pixel is mapped back through the instance's matrix.
fn blit_turned(frame: &mut [u8], b: &[f32], cx: f32, cy: f32, mark: u8, tints: &mut TintCache) {
    let Some(sp) = sprite(b[16] as usize) else {
        return;
    };
    let (sw, sh, pix) = (sp.w, sp.h, sp.pix);
    let tint = b[18] as u32;
    let mut lut = if tint == 0xff_ffff { IDENTITY } else { tints.get(tint) };
    for l in lut.iter_mut() {
        *l |= mark;
    }
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
            if c != CLEAR_PIX {
                frame[y as usize * W + x as usize] = lut[usize::from(c & 15)];
            }
        }
    }
}

/// A square root on the x87's own instruction (the `libm` one is software, and slow when done a thousand times a frame).
fn fsqrt(x: f32) -> f32 {
    let mut r = 0.0f32;
    unsafe {
        core::arch::asm!("fld dword ptr [{x}]", "fsqrt", "fstp dword ptr [{r}]", x = in(reg) &x, r = in(reg) &mut r, options(nostack));
    }
    r
}

/// A colour's brightness as one number.
fn lum(r: f32, g: f32, b: f32) -> f32 {
    0.30 * r + 0.59 * g + 0.11 * b
}

/// Quarter-steps of brightness: 0 is black and `QMAX` full, so a cell's light is `QMAX / 4` levels and a fraction.
const QMAX: f32 = ((LEVELS - 1) * 4) as f32;

/// The light on each 8x8 cell of the screen, in quarter-levels (0..=52): the sim's ambient light, its sun and its
/// point lights, worked out as the web shader does for a sprite facing the screen, with the three colour channels folded
/// into one brightness (the palette can only dim a colour, not tint it). Each point light only visits the cells it can
/// reach.
fn light_levels(out: &[f32], cam_px: (i32, i32)) -> [u8; CELLS_X * CELLS_Y] {
    // The shader lights only when `LIGHTING` is on.
    if out[12] < 0.5 {
        return [QMAX as u8; CELLS_X * CELLS_Y];
    }
    let count = (out[3] as usize).min(16);
    let pos = unsafe { core::slice::from_raw_parts(lights_pos_ptr(), 64) };
    let col = unsafe { core::slice::from_raw_parts(lights_col_ptr(), 48) };
    let (cam_x, cam_y) = (cam_px.0 as f32 / TILE, cam_px.1 as f32 / TILE);
    // A sprite faces the screen, so a sun at (x, y, z) lights it by its z.
    let base = lum(out[9], out[10], out[11]) + lum(out[17], out[18], out[19]) * out[16].max(0.0);
    let mut grid = [base; CELLS_X * CELLS_Y];
    let (half_w, half_h) = (W as f32 / 2.0, H as f32 / 2.0);
    for i in 0..count {
        let (lx, ly, lz, radius) = (pos[i * 4], pos[i * 4 + 1], pos[i * 4 + 2], pos[i * 4 + 3]);
        let strength = lum(col[i * 3], col[i * 3 + 1], col[i * 3 + 2]);
        if radius <= 0.0 || strength <= 0.0 {
            continue;
        }
        // The cells the light reaches, as a box on the screen.
        let sx = half_w + (lx - cam_x) * TILE;
        let sy = half_h - (ly - cam_y) * TILE;
        let reach = radius * TILE;
        let cx0 = (((sx - reach) / CELL as f32) as i32 - 1).max(0) as usize;
        let cx1 = (((sx + reach) / CELL as f32) as i32 + 1).clamp(0, CELLS_X as i32 - 1) as usize;
        let cy0 = (((sy - reach) / CELL as f32) as i32 - 1).max(0) as usize;
        let cy1 = (((sy + reach) / CELL as f32) as i32 + 1).clamp(0, CELLS_Y as i32 - 1) as usize;
        if sx + reach < 0.0 || sy + reach < 0.0 {
            continue;
        }
        for cy in cy0..=cy1 {
            let dy = ly - (cam_y - ((cy * CELL + CELL / 2) as f32 - half_h) / TILE);
            for cx in cx0..=cx1 {
                let dx = lx - (cam_x + ((cx * CELL + CELL / 2) as f32 - half_w) / TILE);
                let d2 = dx * dx + dy * dy + lz * lz;
                if d2 < radius * radius && d2 > 0.0 {
                    let len = fsqrt(d2);
                    let att = 1.0 - len / radius;
                    grid[cy * CELLS_X + cx] += strength * (lz / len).max(0.0) * att * att;
                }
            }
        }
    }
    let mut levels = [0u8; CELLS_X * CELLS_Y];
    for (q, l) in levels.iter_mut().zip(grid.iter()) {
        *q = (l.clamp(0.0, 1.0) * QMAX + 0.5) as u8;
    }
    levels
}

/// Swaps the marker in every frame byte for the brightness level of its cell, and keeps clear-colour pixels as slot 255.
/// A cell's light is in quarter-levels; a 2x2 ordered dither picks the level above or below for each pixel, so the
/// 14 levels do not show as bands or blocks.
fn shade(frame: &mut [u8], levels: &[u8; CELLS_X * CELLS_Y]) {
    // One table per level: frame byte in, final palette index out.
    let mut tables = [[0u8; 256]; LEVELS + 1];
    for (lvl, t) in tables.iter_mut().enumerate().skip(1) {
        for (v, o) in t.iter_mut().enumerate() {
            let (hi, c) = ((v >> 4) as u8, (v & 15) as u8);
            let l = match hi {
                0 => lvl as u8,
                0xD => (lvl as u8).max(ACTOR_FLOOR),
                0xE => LEVELS as u8,
                _ => 0,
            };
            *o = if v == usize::from(CLEAR_SLOT) { CLEAR_SLOT } else { l * 16 + c };
        }
    }
    const BAYER: [[u8; 2]; 2] = [[0, 2], [3, 1]];
    for y in 0..H {
        let row = &mut frame[y * W..][..W];
        let bay = BAYER[y & 1];
        for cx in 0..CELLS_X {
            let q = levels[y / CELL * CELLS_X + cx];
            let (lo, hi) = (
                &tables[usize::from(1 + ((q + bay[0]) >> 2))],
                &tables[usize::from(1 + ((q + bay[1]) >> 2))],
            );
            for pair in row[cx * CELL..][..CELL].chunks_exact_mut(2) {
                pair[0] = lo[usize::from(pair[0])];
                pair[1] = hi[usize::from(pair[1])];
            }
        }
    }
}
