// Drawing for DOS: the sim's instance buffer blitted into a 320x200 8-bit frame (VGA mode 13h).
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT
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
    /// The bits of `w / 16` and `h / 16` as `f32`: an instance drawn at the sprite's own size has these as its scales.
    unit: (u32, u32),
}

/// The sheet's sprites, read once.
const MAX_SPRITES: usize = 256;

fn sheet() -> &'static [Sprite] {
    static mut TABLE: [Sprite; MAX_SPRITES] = [Sprite { w: 0, h: 0, opaque: false, flat: None, pix: &[], unit: (0, 0) }; MAX_SPRITES];
    static mut LEN: usize = usize::MAX;
    // SAFETY: the DOS build has one thread; the table is filled on the first call and only read after.
    unsafe {
        if LEN == usize::MAX {
            let n = u16_at(0).min(MAX_SPRITES);
            let t = &mut *core::ptr::addr_of_mut!(TABLE);
            for (i, s) in t.iter_mut().enumerate().take(n) {
                let o = u32_at(2 + i * 4);
                let (w, h) = (u16_at(o), u16_at(o + 2));
                let flags = SPRITES[o + 4];
                *s = Sprite {
                    w,
                    h,
                    opaque: flags & 1 != 0,
                    flat: (flags & 2 != 0).then(|| SPRITES[o + 5]),
                    pix: &SPRITES[o + 6..o + 6 + w * h],
                    unit: ((w as f32 / TILE).to_bits(), (h as f32 / TILE).to_bits()),
                };
            }
            LEN = n;
        }
        (&*core::ptr::addr_of!(TABLE)).get_unchecked(..LEN)
    }
}

fn sprite(i: usize) -> Option<Sprite> {
    sheet().get(i).copied()
}

/// Starts a level: a new game, the level, the 20x12.5 tile view of a 320x200 screen, and the sprite sizes the
/// sim needs to build its instances (the web shell fills the same table from its atlas).
pub fn start(level: u32) {
    game_new();
    if level == TITLE {
        crate::shell::start_title();
    } else if level >= MAP {
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
pub const MAP: u32 = 254;
/// ... and 255 to mean the title screen.
pub const TITLE: u32 = 255;

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

/// Low detail leaves out the backdrop layers (the sprites named `...Back`): a long way behind, and the dearest thing in a frame.
static LOW_DETAIL: AtomicU32 = AtomicU32::new(0);

pub fn set_low_detail(on: bool) {
    LOW_DETAIL.store(u32::from(on), Ordering::Relaxed);
}

/// Whether sprite `idx` is a backdrop layer, from its name, worked out once.
fn is_backdrop(idx: usize) -> bool {
    static mut BACKDROP: [bool; 512] = [false; 512];
    static mut READY: bool = false;
    unsafe {
        let table = &mut *core::ptr::addr_of_mut!(BACKDROP);
        if !READY {
            for (i, n) in lf_episode_1::sprites::SPRITE_NAMES.iter().enumerate().take(512) {
                table[i] = n.ends_with("Back");
            }
            READY = true;
        }
        idx < 512 && table[idx]
    }
}

pub fn night() -> bool {
    NIGHT.load(Ordering::Relaxed) != 0
}

/// Enters `level` (wrapping round the table).
pub fn goto(level: u32) {
    crate::shell::set_play();
    enter_level(level % TABLE_LEVELS);
}

/// Enters the overworld.
pub fn goto_map() {
    crate::shell::set_play();
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
    if PRESS.load(Ordering::Relaxed) == 3 {
        return 0; // a check that wants no keys pressed at all
    }
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

/// A hook the native benchmark sets to count instructions between the stages of a frame; the DOS build leaves it empty.
static HOOK: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);

/// Sets the profiling hook (a `fn(u8)` called at the end of each stage).
pub fn set_hook(f: fn(u8)) {
    HOOK.store(f as usize, Ordering::Relaxed);
}

/// Marks the end of a stage: 1 the sim's render, 3 the lights, 4 the clear, 5 the sprites, 6 the shading, 7 the overlay,
/// 8 the sim's step, 9 reading its events.
#[inline(always)]
pub fn mark(stage: u8) {
    let h = HOOK.load(Ordering::Relaxed);
    if h != 0 {
        // SAFETY: only `set_hook` stores here, and it stores a `fn(u8)`.
        unsafe { core::mem::transmute::<usize, fn(u8)>(h)(stage) };
    }
}

/// Draws the current scene into `frame` (W*H bytes). Returns the clear colour as 0x00RRGGBB with 6-bit channels,
/// for palette slot 255.
pub fn draw(frame: &mut [u8], alpha: f32) -> u32 {
    let count = build(alpha);
    paint(frame, count)
}

/// The sim writes the scene into its instance buffer; returns how many sprites.
pub fn build(alpha: f32) -> usize {
    let night = if NIGHT.load(Ordering::Relaxed) != 0 { lf_episode_1::render::flags::NIGHT } else { 0 };
    let n = render(f64::from(alpha), lf_episode_1::render::flags::CULLING | night) as usize;
    mark(1);
    n
}

/// Draws what `build` wrote into `frame`.
pub fn paint(frame: &mut [u8], count: usize) -> u32 {
    let out = unsafe { core::slice::from_raw_parts(out_ptr(), 32) };
    // The camera is snapped to whole pixels first: a sprite's place is its own world position rounded, less the camera, so
    // things that are still stay still relative to each other as the camera glides.
    let cam_px = (round(out[1] * TILE), round(out[2] * TILE));
    let six = |v: f32| (v.clamp(0.0, 1.0) * 63.0 + 0.5) as u32;
    let clear = six(out[5]) << 16 | six(out[6]) << 8 | six(out[7]);
    // SAFETY: the DOS build has one thread, and only `paint` touches the still scene.
    let still = unsafe { &mut *core::ptr::addr_of_mut!(STILL) };
    if still.reuse(count, frame) {
        mark(6);
        crate::shell::overlay(frame, cam_px);
        mark(7);
        return clear;
    }
    let levels = light_levels(out, cam_px);
    mark(3);
    // The list is in painter's order and little of the screen is covered twice, so it is drawn back to front.
    frame.fill(CLEAR_SLOT);
    mark(4);
    let inst = unsafe { core::slice::from_raw_parts(instance_ptr(), count * 20) };
    let mut tints = TintCache::new();
    // SAFETY: the DOS build has one thread, and only `paint` touches the cache.
    let bakes = unsafe { &mut *core::ptr::addr_of_mut!(BAKES) };
    // Where the light is the same all over (the map, the day levels) the markers are the final bytes' high nibble
    // already, and no pass is needed after the sprites.
    let uniform = levels.iter().all(|&q| q == levels[0] && q & 3 == 0);
    let marks = if uniform {
        let level = 1 + (levels[0] >> 2);
        [level << 4, level.max(ACTOR_FLOOR) << 4, (LEVELS as u8) << 4]
    } else {
        [0, MARK_ACTOR, MARK_GLOW]
    };
    let low = LOW_DETAIL.load(Ordering::Relaxed) != 0;
    for b in inst.chunks_exact(20) {
        if low && is_backdrop(f32_whole(b[16]) as usize) {
            continue;
        }
        blit(frame, b, cam_px, &mut tints, bakes, &marks);
    }
    mark(5);
    if !uniform {
        shade(frame, &levels);
    }
    still.keep(count, frame);
    mark(6);
    crate::shell::overlay(frame, cam_px);
    mark(7);
    clear
}

/// Sim steps so far, counted by the shell; a scene is only kept while this stands still (a pause, a menu, a card, a dialogue).
static STEPS: AtomicU32 = AtomicU32::new(0);

/// The shell calls this after each step of the sim.
pub fn stepped() {
    STEPS.fetch_add(1, Ordering::Relaxed);
}

/// The most sprites a kept scene can have.
const STILL_MAX: usize = 2048;

/// The last scene drawn while the sim stood still, before the interface went over it, and everything it was drawn from. While
/// the sim is paused (a menu, a result card, a dialogue) the same scene is drawn every frame; when the sim's output, its lights,
/// the draw list and the options are all bit for bit what they were, the kept frame is copied back instead of drawn again.
struct Still {
    valid: bool,
    steps: u32,
    count: usize,
    /// The sim's output block, its lights' places and colours, night and detail.
    head: [u32; 32 + 64 + 48 + 2],
    inst: [u32; STILL_MAX * 20],
    frame: [u8; W * H],
}

static mut STILL: Still = Still { valid: false, steps: 0, count: 0, head: [0; 32 + 64 + 48 + 2], inst: [0; STILL_MAX * 20], frame: [0; W * H] };

impl Still {
    fn head_now() -> [u32; 32 + 64 + 48 + 2] {
        let mut h = [0u32; 32 + 64 + 48 + 2];
        // SAFETY: the sim's output, light places and light colours are 32, 64 and 48 floats.
        let src = unsafe {
            [
                core::slice::from_raw_parts(out_ptr() as *const u32, 32),
                core::slice::from_raw_parts(lights_pos_ptr() as *const u32, 64),
                core::slice::from_raw_parts(lights_col_ptr() as *const u32, 48),
            ]
        };
        for (o, v) in h.iter_mut().zip(src.iter().flat_map(|s| s.iter())) {
            *o = *v;
        }
        h[144] = NIGHT.load(Ordering::Relaxed);
        h[145] = LOW_DETAIL.load(Ordering::Relaxed);
        h
    }

    fn inst_now(count: usize) -> &'static [u32] {
        // SAFETY: the sim's instance buffer holds `count` instances of 20 floats.
        unsafe { core::slice::from_raw_parts(instance_ptr() as *const u32, count * 20) }
    }

    /// Copies the kept scene into `frame` if it is still the scene to draw.
    fn reuse(&mut self, count: usize, frame: &mut [u8]) -> bool {
        let ok = self.valid
            && self.steps == STEPS.load(Ordering::Relaxed)
            && self.count == count
            && self.head == Self::head_now()
            && self.inst[..count * 20] == *Self::inst_now(count);
        if ok {
            frame.copy_from_slice(&self.frame);
        }
        ok
    }

    /// Keeps the scene just drawn when the sim has not stepped since the last frame and its output block is the same as the last
    /// frame's too. A frame that follows no step but moves the camera (the interpolation between ticks, on a machine quicker than
    /// 60 Hz) is never the same again, so copying it would cost 64 KB and the draw list for nothing; the first still frame is only
    /// noted, and a second one in a row is kept.
    fn keep(&mut self, count: usize, frame: &[u8]) {
        let steps = STEPS.load(Ordering::Relaxed);
        let head = Self::head_now();
        let armed = self.steps == steps && self.head == head;
        self.steps = steps;
        self.head = head;
        self.valid = false;
        if !armed || count > STILL_MAX {
            return;
        }
        self.count = count;
        self.inst[..count * 20].copy_from_slice(Self::inst_now(count));
        self.frame.copy_from_slice(frame);
        self.valid = true;
    }
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

/// The largest sprite `BakeCache` keeps: a tile.
const BAKED_PIX: usize = 16 * 16;
/// Sprites `BakeCache` has a slot for (the sheet has 253).
const BAKED: usize = 256;

/// Opaque tiles with their final colours worked out (the tint's colour table and the light marker applied to every pixel), so a
/// tile drawn many times a frame is copied rather than changed a pixel at a time. One slot per sprite, kept from frame to frame: a
/// tile's tint and marker rarely change, so a slot is worked out again only when they do.
struct BakeCache {
    /// The tint and marker each slot was baked with, and bit 40 set; 0 is an empty slot (so the cache starts all zero and takes
    /// no room in the EXE).
    keys: [u64; BAKED],
    pix: [[u8; BAKED_PIX]; BAKED],
}

static mut BAKES: BakeCache = BakeCache { keys: [0; BAKED], pix: [[0; BAKED_PIX]; BAKED] };

impl BakeCache {
    fn get(&mut self, id: usize, tint: u32, mark: u8, sp: &Sprite, lut: &[u8; 16]) -> &[u8; BAKED_PIX] {
        let key = u64::from(tint) | u64::from(mark) << 32 | 1 << 40;
        let i = id % BAKED;
        if self.keys[i] != key {
            self.keys[i] = key;
            for (o, &c) in self.pix[i].iter_mut().zip(sp.pix) {
                *o = lut[usize::from(c & 15)];
            }
        }
        &self.pix[i]
    }
}

/// Copies a row of a sprite, with the common whole tile row (16 bytes) as four word moves rather than a call to `memcpy`.
#[inline(always)]
fn copy_row(dst: &mut [u8], src: &[u8]) {
    if let (Ok(d), Ok(s)) = (<&mut [u8; 16]>::try_from(&mut *dst), <&[u8; 16]>::try_from(src)) {
        *d = *s;
    } else {
        dst.copy_from_slice(src);
    }
}

/// `v` rounded to the nearest integer (positions here are above -1000, so no floor call is needed).
fn round(v: f32) -> i32 {
    (v + 1000.5) as i32 - 1000
}

const IDENTITY: [u8; 16] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];

/// `floor(v * 2^k)`, worked out exactly from the bits of `v` with integers (an x87 conversion costs a control-word swap and a
/// `fistp`, some 40 cycles on a 486). Values too big for the screen come back as +-2^30.
fn floor_scaled(v: f32, k: i32) -> i32 {
    let b = v.to_bits();
    let neg = b >> 31 != 0;
    let e = ((b >> 23) & 255) as i32;
    let m = if e == 0 { (b & 0x7f_ffff) as i32 } else { ((b & 0x7f_ffff) | 0x80_0000) as i32 };
    if m == 0 {
        return 0;
    }
    // v = m * 2^sh, after the scaling by 2^k.
    let sh = e.max(1) - 150 + k;
    if sh > 6 {
        return if neg { -(1 << 30) } else { 1 << 30 };
    }
    if sh >= 0 {
        return if neg { -(m << sh) } else { m << sh };
    }
    if sh <= -31 {
        return if neg { -1 } else { 0 };
    }
    if neg {
        -((m + (1 << -sh) - 1) >> -sh)
    } else {
        m >> -sh
    }
}

/// A whole number the sim stored as an `f32` (a sprite index, a tint), read from its bits; as `v as u32` for 0 <= v < 2^24.
fn f32_whole(v: f32) -> u32 {
    let b = v.to_bits();
    let e = (b >> 23) as i32 - 127; // the sign bit makes a negative value's `e` large, and it is read as 0
    if !(0..=23).contains(&e) {
        return 0;
    }
    ((b & 0x7f_ffff) | 0x80_0000) >> (23 - e)
}

/// `f32` bits of the flag thresholds (for positive floats the order of the bits is the order of the values).
const F0_5: u32 = 0.5f32.to_bits();
const F1_5: u32 = 1.5f32.to_bits();
const F2_5: u32 = 2.5f32.to_bits();
const F3_5: u32 = 3.5f32.to_bits();
const F4_5: u32 = 4.5f32.to_bits();
const F5_5: u32 = 5.5f32.to_bits();
const F6_5: u32 = 6.5f32.to_bits();

fn blit(frame: &mut [u8], b: &[f32], cam_px: (i32, i32), tints: &mut TintCache, bakes: &mut BakeCache, marks: &[u8; 3]) {
    // Alpha, emissive and actor share one float: alpha + 2 * emissive + 4 * actor. A sprite under half alpha is not drawn. They
    // are read from the bits, which compare as the values do: with w2 the float less the actor's 4, the sprite is drawn when w2
    // is in [0.5, 1.5) or at least 2.5, and glows when w2 is 1.5 or more.
    let fb = b[19].to_bits();
    if fb >> 31 != 0 {
        return;
    }
    let actor = fb >= F3_5;
    let (shown, glow) = if actor {
        ((F4_5..F5_5).contains(&fb) || fb >= F6_5, fb >= F5_5)
    } else {
        ((F0_5..F1_5).contains(&fb) || (F2_5..F3_5).contains(&fb), fb >= F1_5)
    };
    if !shown {
        return;
    }
    let mark = if glow {
        marks[2]
    } else if actor {
        marks[1]
    } else {
        marks[0]
    };
    let (m00, m01, m10, m11) = (b[0], b[1], b[4], b[5]);
    // Most sprites are not turned: they are copied, flipped or scaled by whole steps of fixed-point, with no
    // floating-point work per pixel (the x87 is slow, and DOSBox's emulation of it slower).
    if (m01.to_bits() | m10.to_bits()) << 1 == 0 {
        let Some(sp) = sprite(f32_whole(b[16]) as usize) else {
            return;
        };
        let (sw, sh) = (sp.w, sp.h);
        let (x0, y0, dw, dh);
        if (m00.to_bits() & 0x7fff_ffff, m11.to_bits() & 0x7fff_ffff) == sp.unit {
            // At its own size, the usual case, the place comes from the bits with no x87 work: the left edge is
            // `round(x * 16 - w / 2)`, and from f = floor(x * 32) that is `(f + 1) >> 1` less w / 2 for an even width, or
            // `f >> 1` less (w - 1) / 2 for an odd one; the top likewise.
            let (fx, fy) = (floor_scaled(b[12], 5), floor_scaled(b[13], 5));
            let (w, h) = (sw as i32, sh as i32);
            x0 = (if w & 1 == 0 { ((fx + 1) >> 1) - w / 2 } else { (fx >> 1) - (w - 1) / 2 }) + W as i32 / 2 - cam_px.0;
            y0 = H as i32 / 2 + cam_px.1 - if h & 1 == 0 { ((fy + 1) >> 1) + h / 2 } else { (fy >> 1) + (h + 1) / 2 };
            (dw, dh) = (w, h);
        } else {
            (dw, dh) = (round(m00.abs() * TILE), round(m11.abs() * TILE));
            if dw <= 0 || dh <= 0 {
                return;
            }
            // Rounded in the world first, then moved by the whole-pixel camera.
            x0 = round(b[12] * TILE - m00.abs() * TILE * 0.5) + W as i32 / 2 - cam_px.0;
            y0 = H as i32 / 2 + cam_px.1 - round(b[13] * TILE + m11.abs() * TILE * 0.5);
        }
        if x0 >= W as i32 || y0 >= H as i32 || x0 + dw <= 0 || y0 + dh <= 0 {
            return;
        }
        let tint = f32_whole(b[18]);
        let identity = tint == 0xff_ffff;
        let mut lut = if tint == 0xff_ffff { IDENTITY } else { tints.get(tint) };
        for l in lut.iter_mut() {
            *l |= mark;
        }
        // From the sign bits (neither scale is zero here), so no x87 compare is left inside the row loops.
        let (flip_x, flip_y) = (m00.to_bits() >> 31 != 0, m11.to_bits() >> 31 != 0);
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
            // An opaque tile is copied a row at a time: as it is when it needs no change, else from its baked copy (the backdrop
            // layers repeat one tinted tile a few hundred times, and a level with even light marks every tile).
            if sp.opaque && !flip_x && sw * sh <= BAKED_PIX {
                let pix: &[u8] = if identity && mark == 0 { sp.pix } else { bakes.get(f32_whole(b[16]) as usize, tint, mark, &sp, &lut) };
                for y in cy0..cy1 {
                    let r = if flip_y { sh - 1 - (y - y0) as usize } else { (y - y0) as usize };
                    copy_row(&mut frame[y as usize * W + cx0 as usize..][..span], &pix[r * sw + skip..][..span]);
                }
                return;
            }
            for y in cy0..cy1 {
                let mut r = (y - y0) as usize;
                if flip_y {
                    r = sh - 1 - r;
                }
                let src = &sp.pix[r * sw..][..sw];
                let dst = &mut frame[y as usize * W + cx0 as usize..][..span];
                if !flip_x {
                    let s = &src[skip..][..span];
                    if sp.opaque && identity && mark == 0 {
                        dst.copy_from_slice(s);
                    } else if sp.opaque && identity {
                        // The colours are 0..15 and the mark is a high nibble, so four at a time add without carrying.
                        let add = u32::from(mark) * 0x0101_0101;
                        let mut d4 = dst.chunks_exact_mut(4);
                        let mut s4 = s.chunks_exact(4);
                        for (d, c) in (&mut d4).zip(&mut s4) {
                            let w = u32::from_le_bytes([c[0], c[1], c[2], c[3]]) + add;
                            d.copy_from_slice(&w.to_le_bytes());
                        }
                        for (d, &c) in d4.into_remainder().iter_mut().zip(s4.remainder()) {
                            *d = c + mark;
                        }
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
    let cx = W as f32 / 2.0 + b[12] * TILE - cam_px.0 as f32;
    let cy = H as f32 / 2.0 - b[13] * TILE + cam_px.1 as f32;
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

/// A square root on the x87's own instruction (the `libm` one is software); only the falloff table's set-up uses it.
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
const QMAX: i32 = ((LEVELS - 1) * 4) as i32;

/// Light in the grid is fixed point: `LIGHT_ONE` is full brightness.
const LIGHT_SHIFT: u32 = 10;
const LIGHT_ONE: i32 = 1 << LIGHT_SHIFT;
/// Steps of the falloff table, over the squared distance from 0 to the light's radius.
const FALL_STEPS: u32 = 1024;
/// Falloff table entries are this many times the value they stand for.
const FALL_ONE: f32 = 256.0;

/// The falloff of a point light against its squared distance: for `u` the distance over the radius, `(1 - u)^2 / u` (the web
/// shader's `(1 - u)^2` times the `z / distance` of a sprite facing the screen, with `z / radius` taken out), at the middle of
/// each of `FALL_STEPS` steps of `u^2`, times `FALL_ONE`. A light is never nearer than its own height, so the capped first steps
/// are rarely used.
fn falloff() -> &'static [u16; FALL_STEPS as usize] {
    static mut TABLE: [u16; FALL_STEPS as usize] = [0; FALL_STEPS as usize];
    static mut BUILT: bool = false;
    unsafe {
        let t = &mut *core::ptr::addr_of_mut!(TABLE);
        if !BUILT {
            for (i, o) in t.iter_mut().enumerate() {
                let u = fsqrt((i as f32 + 0.5) / FALL_STEPS as f32);
                *o = ((1.0 - u) * (1.0 - u) / u * FALL_ONE + 0.5).min(65535.0) as u16;
            }
            BUILT = true;
        }
        &*core::ptr::addr_of!(TABLE)
    }
}

/// The light on each 8x8 cell of the screen, in quarter-levels (0..=52): the sim's ambient light, its sun and its
/// point lights, worked out as the web shader does for a sprite facing the screen, with the three colour channels folded
/// into one brightness (the palette can only dim a colour, not tint it). Each point light only visits the cells it can
/// reach, in integers: distances are in quarter pixels and the falloff comes from a table, so a cell costs a few integer
/// steps where a square root and two divisions on the x87 would cost several hundred cycles on a 486.
fn light_levels(out: &[f32], cam_px: (i32, i32)) -> [u8; CELLS_X * CELLS_Y] {
    // The shader lights only when `LIGHTING` is on.
    if out[12] < 0.5 {
        return [QMAX as u8; CELLS_X * CELLS_Y];
    }
    let count = (out[3] as usize).min(16);
    let pos = unsafe { core::slice::from_raw_parts(lights_pos_ptr(), 64) };
    let col = unsafe { core::slice::from_raw_parts(lights_col_ptr(), 48) };
    let fall = falloff();
    // A sprite faces the screen, so a sun at (x, y, z) lights it by its z.
    let base = lum(out[9], out[10], out[11]) + lum(out[17], out[18], out[19]) * out[16].max(0.0);
    let mut grid = [(base.clamp(0.0, 4.0) * LIGHT_ONE as f32 + 0.5) as i32; CELLS_X * CELLS_Y];
    /// Quarter pixels to a tile, and to a cell.
    const SUB: f32 = TILE * 4.0;
    const CELL_SUB: i32 = CELL as i32 * 4;
    for i in 0..count {
        let (lx, ly, lz, radius) = (pos[i * 4], pos[i * 4 + 1], pos[i * 4 + 2], pos[i * 4 + 3].min(63.0));
        let strength = lum(col[i * 3], col[i * 3 + 1], col[i * 3 + 2]);
        // A light at or behind the sprites' plane adds nothing (the shader's `z / distance` is clamped at 0).
        if radius <= 0.0 || strength <= 0.0 || lz <= 0.0 {
            continue;
        }
        // The light's place on the screen, in quarter pixels from the top left.
        let sx = ((W as i32 / 2 - cam_px.0) * 4) as f32 + lx * SUB;
        let sy = ((H as i32 / 2 + cam_px.1) * 4) as f32 - ly * SUB;
        if !(-1.0e6..1.0e6).contains(&sx) || !(-1.0e6..1.0e6).contains(&sy) {
            continue;
        }
        let (sx, sy) = (sx as i32, sy as i32);
        let r = (radius * SUB) as i32;
        let z = (lz * SUB) as i32;
        let (r2, z2) = (r * r, z * z);
        if z2 >= r2 {
            continue;
        }
        // The cells the light reaches, as a box on the screen.
        let cx0 = ((sx - r).div_euclid(CELL_SUB)).max(0);
        let cx1 = ((sx + r).div_euclid(CELL_SUB)).min(CELLS_X as i32 - 1);
        let cy0 = ((sy - r).div_euclid(CELL_SUB)).max(0);
        let cy1 = ((sy + r).div_euclid(CELL_SUB)).min(CELLS_Y as i32 - 1);
        // What a cell gets is `k * falloff >> 8`, with k the strength times z / radius in grid units; the table index is the
        // squared distance in steps of r2 / FALL_STEPS, and as d2 < r2 the product `d2 * step` stays under 2^26.
        let k = (strength * lz / radius * LIGHT_ONE as f32 + 0.5) as i32;
        let step = (FALL_STEPS << 16) / r2 as u32;
        for cy in cy0..=cy1 {
            let dy = cy * CELL_SUB + CELL_SUB / 2 - sy;
            let dyz = dy * dy + z2;
            if dyz >= r2 {
                continue;
            }
            let row = &mut grid[cy as usize * CELLS_X..][..CELLS_X];
            for cx in cx0..=cx1 {
                let dx = cx * CELL_SUB + CELL_SUB / 2 - sx;
                let d2 = dx * dx + dyz;
                if d2 < r2 {
                    let f = i32::from(fall[((d2 as u32 * step) >> 16) as usize]);
                    row[cx as usize] += (k * f + 128) >> 8;
                }
            }
        }
    }
    let mut levels = [0u8; CELLS_X * CELLS_Y];
    for (q, &l) in levels.iter_mut().zip(grid.iter()) {
        *q = ((l.clamp(0, LIGHT_ONE) * QMAX + LIGHT_ONE / 2) >> LIGHT_SHIFT) as u8;
    }
    levels
}

/// Swaps the marker in every frame byte for the brightness level of its cell, and keeps clear-colour pixels as slot 255.
/// A cell's light is in quarter-levels; a 2x2 ordered dither picks the level above or below for each pixel, so the
/// 14 levels do not show as bands or blocks.
fn shade(frame: &mut [u8], levels: &[u8; CELLS_X * CELLS_Y]) {
    // One table per level: frame byte in, final palette index out. They never change, so they are built once.
    static mut TABLES: [[u8; 256]; LEVELS + 1] = [[0; 256]; LEVELS + 1];
    static mut BUILT: bool = false;
    let tables = unsafe {
        let t = &mut *core::ptr::addr_of_mut!(TABLES);
        if !BUILT {
            for (lvl, t) in t.iter_mut().enumerate().skip(1) {
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
            BUILT = true;
        }
        &*core::ptr::addr_of!(TABLES)
    };
    const BAYER: [[u8; 2]; 2] = [[0, 2], [3, 1]];
    for cy in 0..CELLS_Y {
        // What each cell adds to a lit pixel, for the two kinds of row (the dither swaps the levels along them): four bytes
        // alternating `lo << 4` and `hi << 4`, the levels of the even and odd pixels.
        let mut adds = [[0u32; CELLS_X]; 2];
        for (parity, bay) in BAYER.iter().enumerate() {
            for cx in 0..CELLS_X {
                let q = levels[cy * CELLS_X + cx];
                let (l0, l1) = (u32::from(1 + ((q + bay[0]) >> 2)), u32::from(1 + ((q + bay[1]) >> 2)));
                adds[parity][cx] = l0 << 4 | l1 << 12 | l0 << 20 | l1 << 28;
            }
        }
        for r in 0..CELL {
            let y = cy * CELL + r;
            let row = &mut frame[y * W..][..W];
            let add_row = &adds[y & 1];
            let base = row.as_mut_ptr() as *mut u32;
            for k in 0..W / 4 {
                let add = add_row[k >> 1];
                // SAFETY: `k` is under the row's width in words, so the word is inside the row; x86 reads and writes unaligned freely.
                let w = unsafe { base.add(k).read_unaligned() };
                let marks = w & 0xf0f0_f0f0;
                if marks == 0 {
                    // A lit pixel is its colour 0..15 with a zero high nibble, so four take their level in one add.
                    unsafe { base.add(k).write_unaligned(w + add) };
                } else if marks != 0xe0e0_e0e0 && w != 0xffff_ffff {
                    // (Four glowing pixels, or four clear ones, are already final.)
                    unsafe { base.add(k).write_unaligned(shade_word(w, add, tables)) };
                }
            }
        }
    }
}

/// Four pixels that are not all lit: glowing ones are already at the top level and take no add; an actor or a clear pixel
/// goes through the tables.
#[inline(never)]
fn shade_word(w: u32, add: u32, tables: &[[u8; 256]; LEVELS + 1]) -> u32 {
    let u = (w >> 5) & 0x0707_0707;
    if w & 0x1010_1010 == 0 && u == (u & 0x0101_0101) * 7 {
        // Lit and glowing pixels only: spread the glowing bytes' 0xE0 into a full mask and leave those bytes out of the add.
        let g = w & 0xe0e0_e0e0;
        let g = g | (g >> 3);
        return w + (add & !(g | ((g >> 6) & 0x0303_0303)));
    }
    let (lo, hi) = (&tables[((add >> 4) & 15) as usize], &tables[((add >> 12) & 15) as usize]);
    u32::from(lo[(w & 255) as usize])
        | u32::from(hi[((w >> 8) & 255) as usize]) << 8
        | u32::from(lo[((w >> 16) & 255) as usize]) << 16
        | u32::from(hi[(w >> 24) as usize]) << 24
}

/// Draws sprite `id` for the interface (full brightness, no lighting) with its top left at (`x`, `y`).
pub fn draw_sprite_ui(frame: &mut [u8], id: usize, x: i32, y: i32) {
    draw_sprite_ui_flip(frame, id, x, y, false);
}

/// The sheet's id for a sprite name the sim draws, if there is one.
pub fn sprite_by_name(name: &str) -> Option<usize> {
    lf_episode_1::sprites::SPRITE_NAMES.iter().position(|n| *n == name)
}

/// The height of sprite `id` in pixels (0 if there is none).
pub fn sprite_height(id: usize) -> i32 {
    sprite(id).map_or(0, |sp| sp.h as i32)
}

/// `draw_sprite_ui`, mirrored left to right when `flip` is set.
pub fn draw_sprite_ui_flip(frame: &mut [u8], id: usize, x: i32, y: i32, flip: bool) {
    let Some(sp) = sprite(id) else {
        return;
    };
    // The column to read for the first pixel of a row, and the step along it.
    let (first, step) = if flip { (sp.w as i32 - 1, -1) } else { (0, 1) };
    for row in 0..sp.h as i32 {
        for col in 0..sp.w as i32 {
            let c = sp.pix[row as usize * sp.w + (first + step * col) as usize];
            let (px, py) = (x + col, y + row);
            if c != CLEAR_PIX && px >= 0 && py >= 0 && (px as usize) < W && (py as usize) < H {
                frame[py as usize * W + px as usize] = crate::ui::FULL + (c & 15);
            }
        }
    }
}
