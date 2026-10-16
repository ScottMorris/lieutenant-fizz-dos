// Drawing for DOS: the sim's instance buffer blitted into a 320x200 8-bit frame (VGA mode 13h).
//
// Sprites come from `m3/assets/sprites.bin` (see m3/export-sprites.ts): 0 is transparent, 1..16 the EGA colours.
// The frame uses palette slots 1..16 for the sprites and 17 for the level's clear colour.

use lf_episode_1::{
    enter_level, game_new, instance_ptr, out_ptr, render, set_view, sprite_table_ptr, step,
    table_len,
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

/// Sprite `i`: its width, height and pixels.
fn sprite(i: usize) -> Option<(usize, usize, &'static [u8])> {
    if i >= u16_at(0) {
        return None;
    }
    let o = u32_at(2 + i * 4);
    let (w, h) = (u16_at(o), u16_at(o + 2));
    Some((w, h, &SPRITES[o + 4..o + 4 + w * h]))
}

/// Starts a level: a new game, the level, the 20x12.5 tile view of a 320x200 screen, and the sprite sizes the
/// sim needs to build its instances (the web shell fills the same table from its atlas).
pub fn start(level: u32) {
    game_new();
    enter_level(level);
    set_view(f64::from(W as u32) / 32.0, f64::from(H as u32) / 32.0);
    let n = table_len(0) as usize;
    let table = unsafe { core::slice::from_raw_parts_mut(sprite_table_ptr(), n * 6) };
    for (i, s) in table.chunks_exact_mut(6).enumerate() {
        let (w, h) = sprite(i).map_or((1, 1), |(w, h, _)| (w, h));
        s.copy_from_slice(&[i as f32, 0.0, 1.0, 1.0, w as f32, h as f32]);
    }
}

/// A scripted walk to the right with a jump now and then.
pub fn scripted_input(tick: u32) -> u32 {
    const RIGHT: u32 = 2;
    const JUMP: u32 = 16;
    RIGHT | if tick % 45 < 14 { JUMP } else { 0 }
}

pub fn advance(held: u32) {
    step(held);
}

/// Draws the current scene into `frame` (W*H bytes). Returns the clear colour as 0x00RRGGBB with 6-bit channels,
/// for palette slot 17.
pub fn draw(frame: &mut [u8]) -> u32 {
    let count = render(1.0, 0) as usize;
    let out = unsafe { core::slice::from_raw_parts(out_ptr(), 32) };
    let (cam_x, cam_y) = (out[1], out[2]);
    let six = |v: f32| (v.clamp(0.0, 1.0) * 63.0 + 0.5) as u32;
    let clear = six(out[5]) << 16 | six(out[6]) << 8 | six(out[7]);
    frame.fill(CLEAR_SLOT);
    let inst = unsafe { core::slice::from_raw_parts(instance_ptr(), count * 20) };
    for b in inst.chunks_exact(20) {
        blit(frame, b, cam_x, cam_y);
    }
    clear
}

fn blit(frame: &mut [u8], b: &[f32], cam_x: f32, cam_y: f32) {
    // Alpha, emissive and actor share one float: alpha + 2 * emissive + 4 * actor.
    let flags = b[19];
    let w2 = if flags >= 3.5 { flags - 4.0 } else { flags };
    let alpha = if w2 >= 1.5 { w2 - 2.0 } else { w2 };
    if alpha < 0.5 {
        return;
    }
    let Some((sw, sh, pix)) = sprite(b[16] as usize) else {
        return;
    };
    let tint = b[18] as u32;
    let lut = if tint == 0xff_ffff { None } else { Some(tint_lut(tint)) };
    let (m00, m01, m10, m11) = (b[0], b[1], b[4], b[5]);
    let det = m00 * m11 - m10 * m01;
    if det == 0.0 {
        return;
    }
    // The quad's centre on the screen, and the box that holds its (possibly rotated) corners.
    let cx = W as f32 / 2.0 + (b[12] - cam_x) * TILE;
    let cy = H as f32 / 2.0 - (b[13] - cam_y) * TILE;
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
                frame[y as usize * W + x as usize] = lut.map_or(c, |l| l[usize::from(c)]);
            }
        }
    }
}
