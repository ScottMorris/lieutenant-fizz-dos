// The opening cinematic's pictures: the backyard, the secret lab, the liftoff and the flight to Zargoth.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// A port of the main game's `cine.ts`. The scenes are the same sprites in the same places, drawn at half the usual tile size so the
// 320 × 200 screen shows 40 × 25 tiles, through `video::draw_cine`: hue picks the nearest EGA colour and brightness a palette level, so
// the dark blues of the night are dim blues. Alpha is a threshold (a sprite under half is left out), and the lighting pass is not run.

use crate::video::{self, Inst};

/// Tiles to half-size tiles.
const Z: f32 = 0.5;
const GROUND: f32 = -2.5;
const STARS: usize = 260;
const PUFFS: usize = 32;
const DT: f32 = 1.0 / 60.0;
/// The sprites the sim never draws, which `m3/export-sprites.ts` puts after its own.
const EXTRA: [&str; 12] = ["planet", "house", "bigTree", "moon", "fence", "labPanel", "console0", "console1", "blueprint", "bench", "pad", "lamp"];

/// A deterministic hash in [0, 1), the sim's `hashf`.
fn hashf(x: i32, y: i32) -> f32 {
    let mut h = x.wrapping_mul(374_761_393).wrapping_add(y.wrapping_mul(668_265_263));
    h = (h ^ ((h as u32) >> 13) as i32).wrapping_mul(1_274_126_177);
    ((h ^ ((h as u32) >> 16) as i32) as u32) as f32 / 4_294_967_296.0
}

fn floor(v: f32) -> f32 {
    let i = v as i32 as f32;
    if i > v {
        i - 1.0
    } else {
        i
    }
}

fn rem(a: f32, b: f32) -> f32 {
    a - b * floor(a / b)
}

/// A sine, close to a percent (Bhaskara's), for the bobbing and the twinkling.
fn sin(x: f32) -> f32 {
    const PI: f32 = 3.141_592_7;
    let x = rem(x + PI, 2.0 * PI) - PI;
    let (a, sign) = if x < 0.0 { (-x, -1.0) } else { (x, 1.0) };
    sign * 16.0 * a * (PI - a) / (5.0 * PI * PI - 4.0 * a * (PI - a))
}

fn sprite_id(name: &str) -> usize {
    let names = lf_episode_1::sprites::SPRITE_NAMES;
    names
        .iter()
        .position(|n| *n == name)
        .or_else(|| EXTRA.iter().position(|n| *n == name).map(|i| names.len() + i))
        .unwrap_or(0)
}

#[derive(Clone, Copy)]
struct Star {
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Clone, Copy)]
struct Puff {
    x: f32,
    y: f32,
    t: f32,
    life: f32,
    vx: f32,
    vy: f32,
}

/// What a sprite is pushed with: the size as a multiple of its own, a turn, a tint and an alpha.
#[derive(Clone, Copy)]
struct Opt {
    s: f32,
    sx: f32,
    sy: f32,
    rot: f32,
    tint: u32,
    alpha: f32,
    flip: bool,
}

const D: Opt = Opt { s: 1.0, sx: 1.0, sy: 1.0, rot: 0.0, tint: 0xff_ffff, alpha: 1.0, flip: false };

struct Ids {
    star: usize,
    puff: usize,
    fill: usize,
    saucer: usize,
    hill_top: usize,
    ladder: usize,
    run: [usize; 2],
    stand: usize,
    planet: usize,
    house: usize,
    big_tree: usize,
    moon: usize,
    fence: usize,
    lab_panel: usize,
    console: [usize; 2],
    blueprint: usize,
    bench: usize,
    pad: usize,
    lamp: usize,
}

impl Ids {
    fn find() -> Ids {
        Ids {
            star: sprite_id("star"),
            puff: sprite_id("puff"),
            fill: sprite_id("fill"),
            saucer: sprite_id("saucer"),
            hill_top: sprite_id("hillTop"),
            ladder: sprite_id("ladder"),
            run: [sprite_id("ben_run1"), sprite_id("ben_run2")],
            stand: sprite_id("ben_stand"),
            planet: sprite_id("planet"),
            house: sprite_id("house"),
            big_tree: sprite_id("bigTree"),
            moon: sprite_id("moon"),
            fence: sprite_id("fence"),
            lab_panel: sprite_id("labPanel"),
            console: [sprite_id("console0"), sprite_id("console1")],
            blueprint: sprite_id("blueprint"),
            bench: sprite_id("bench"),
            pad: sprite_id("pad"),
            lamp: sprite_id("lamp"),
        }
    }
}

pub struct Cine {
    t: f32,
    stage: u8,
    st_t: f32,
    held: bool,
    stars: [Star; STARS],
    fx: [Puff; PUFFS],
    nfx: usize,
    sx: f32,
    sy: f32,
    ly: f32,
    scroll: f32,
    rolls: i32,
    ids: Ids,
}

impl Cine {
    pub fn new() -> Cine {
        let mut stars = [Star { x: 0.0, y: 0.0, z: 0.0 }; STARS];
        for (i, st) in stars.iter_mut().enumerate() {
            let i = i as i32;
            *st = Star { x: (hashf(i, 1) - 0.5) * 60.0, y: (hashf(i, 2) - 0.5) * 30.0, z: 0.2 + hashf(i, 3) * 0.8 };
        }
        Cine {
            t: 0.0,
            stage: 0,
            st_t: 0.0,
            held: false,
            stars,
            fx: [Puff { x: 0.0, y: 0.0, t: 0.0, life: 1.0, vx: 0.0, vy: 0.0 }; PUFFS],
            nfx: 0,
            sx: 0.0,
            sy: 2.0,
            ly: -4.4,
            scroll: 0.0,
            rolls: 0,
            ids: Ids::find(),
        }
    }

    /// Switches to a scene (0 yard, 1 lab, 2 liftoff, 3 to 7 the flight and the landing).
    pub fn start(&mut self, stage: u8) {
        self.stage = stage;
        self.st_t = 0.0;
        self.held = stage == 2;
        self.ly = -4.4;
        self.scroll = 0.0;
        if stage == 3 {
            self.sx = 0.0;
            self.sy = 0.5;
            self.nfx = 0;
        }
    }

    /// Lets the liftoff scene's hatch open and the saucer rise.
    pub fn launch(&mut self) {
        self.held = false;
    }

    fn rand(&mut self) -> f32 {
        self.rolls += 1;
        hashf(self.rolls, 77)
    }

    fn add_puff(&mut self, p: Puff) {
        if self.nfx < PUFFS {
            self.fx[self.nfx] = p;
            self.nfx += 1;
        }
    }

    /// One tick (a sixtieth of a second).
    pub fn tick(&mut self) {
        let dt = DT;
        self.t += dt;
        if !self.held {
            self.st_t += dt;
        }
        let mut keep = 0;
        for i in 0..self.nfx {
            let mut f = self.fx[i];
            f.t += dt;
            f.x += f.vx * dt;
            f.y += f.vy * dt;
            if f.t < f.life {
                self.fx[keep] = f;
                keep += 1;
            }
        }
        self.nfx = keep;
        let tick20 = floor(self.t * 20.0) != floor((self.t - dt) * 20.0);
        if self.stage == 2 {
            let big_t = self.st_t;
            let prev = self.ly;
            let up = (big_t - 1.3).max(0.0);
            self.ly = -4.4 + up * up * 1.7;
            self.scroll = (self.ly - 0.5).max(0.0);
            let v = (self.ly - prev) / dt.max(1e-4);
            if big_t > 1.3 && self.ly > -3.2 && tick20 {
                let (a, b) = (self.rand(), self.rand());
                self.add_puff(Puff {
                    x: (a - 0.5) * 1.4,
                    y: self.ly - self.scroll - 0.7,
                    t: 0.0,
                    life: 0.9,
                    vx: (b - 0.5) * 2.0,
                    vy: -2.0 - if self.scroll > 0.0 { v } else { 0.0 },
                });
            }
            return;
        }
        if self.stage < 2 {
            return;
        }
        let os = self.stage - 2;
        let sp: [f32; 2] = match os {
            0 => [0.0, 0.3],
            1 => [0.0, -8.0],
            2 => [-3.0, 0.0],
            3 => [-40.0, 0.0],
            4 => [-6.0, 0.0],
            _ => [-1.0, 0.5],
        };
        for st in self.stars.iter_mut() {
            st.x += sp[0] * st.z * dt;
            st.y += sp[1] * st.z * dt;
            if st.x < -30.0 {
                st.x += 60.0;
            }
            if st.x > 30.0 {
                st.x -= 60.0;
            }
            if st.y < -15.0 {
                st.y += 30.0;
            }
            if st.y > 15.0 {
                st.y -= 30.0;
            }
        }
        let aim = if os == 5 { 2.6 - self.st_t.min(4.0) * 0.3 } else { 3.0 };
        self.sy += (aim - self.sy) * (dt * 1.5).min(1.0);
        if tick20 {
            if os == 1 {
                let a = self.rand();
                self.add_puff(Puff { x: self.sx + (a - 0.5) * 1.2, y: self.sy - 0.7, t: 0.0, life: 0.8, vx: 0.0, vy: -5.0 });
            } else {
                let wob = sin(self.t * 9.0) * 0.1;
                self.add_puff(Puff { x: self.sx - 1.6, y: self.sy - 0.1 + wob, t: 0.0, life: 0.8, vx: -6.0, vy: 0.0 });
            }
        }
    }

    /// Draws sprite `id` at (x, y) tiles (before the zoom) with `o`, `oy` tiles lower.
    fn put(&self, f: &mut [u8], oy: f32, x: f32, y: f32, id: usize, o: Opt) {
        if o.alpha < 0.5 {
            return;
        }
        let (w, h) = video::sprite_size(id);
        let sx = w as f32 / 16.0 * o.s * o.sx * if o.flip { -1.0 } else { 1.0 } * Z;
        let sy = h as f32 / 16.0 * o.s * o.sy * Z;
        video::draw_cine(f, id, &Inst { x: x * Z, y: (y - oy) * Z, sx, sy, rot: o.rot, tint: o.tint });
    }

    /// The whole picture, from black.
    pub fn draw(&self, f: &mut [u8]) {
        f.fill(0);
        let ids = &self.ids;
        match self.stage {
            0 => self.yard(f, 0.0, false),
            1 => self.lab(f),
            2 => self.yard(f, self.scroll, true),
            stage => {
                let os = stage - 2;
                for st in self.stars.iter() {
                    let str_x = if os == 3 { 1.0 + st.z * 10.0 } else { 1.0 };
                    let str_y = if os == 1 { 1.0 + st.z * 3.0 } else { 1.0 };
                    let tint = if st.z > 0.8 { 0xff_ff55 } else { 0xff_ffff };
                    self.put(f, 0.0, st.x, st.y, ids.star, Opt { s: 0.6 + st.z * 0.6, sx: str_x, sy: str_y, alpha: 0.5 + st.z * 0.5, tint, ..D });
                }
                if os >= 4 {
                    let k = if os == 4 { (self.st_t / 6.0).min(1.0) } else { 1.0 };
                    let sc = if os == 4 { 1.5 + k * 3.5 } else { 9.0 };
                    let px = if os == 4 { 9.0 - k * 3.0 } else { 0.0 };
                    let py = if os == 4 { 3.5 } else { -10.0 - self.st_t.min(4.0) * 0.2 };
                    self.put(f, 0.0, px, py, ids.planet, Opt { s: sc, ..D });
                }
            }
        }
        for p in &self.fx[..self.nfx] {
            self.put(f, 0.0, p.x, p.y, ids.puff, Opt { s: 0.6 + p.t, alpha: 1.0 - p.t / p.life, tint: 0xff_5555, ..D });
        }
        if self.stage >= 3 {
            let rot = if self.stage == 7 { -0.08 } else { sin(self.t * 1.3) * 0.03 };
            self.put(f, 0.0, self.sx, self.sy + sin(self.t * 2.2) * 0.12, ids.saucer, Opt { s: 1.6, rot, ..D });
        }
    }

    fn yard(&self, f: &mut [u8], oy: f32, launch: bool) {
        let (t, big_t, g) = (self.t, self.st_t, GROUND);
        let ids = &self.ids;
        let fl = ids.fill;
        macro_rules! p {
            ($x:expr, $y:expr, $id:expr, $o:expr) => {
                self.put(f, oy, $x, $y, $id, $o)
            };
        }
        // The sky stays where it is as the saucer climbs; the rest goes down with the scroll.
        self.put(f, 0.0, 0.0, 0.0, fl, Opt { sx: 40.0, sy: 20.0, tint: 0x07_0718, ..D });
        p!(0.0, g + 2.4, fl, Opt { sx: 40.0, sy: 3.0, tint: 0x11_0d2c, ..D });
        p!(0.0, g + 1.1, fl, Opt { sx: 40.0, sy: 1.4, tint: 0x22_143a, ..D });
        for st in self.stars.iter() {
            let yy = rem(rem(st.y + 15.0 - oy * 0.3 * st.z, 30.0) + 30.0, 30.0) - 15.0;
            if yy > g - oy + 1.4 {
                let twinkle = 0.35 + 0.45 * (0.5 + 0.5 * sin(t * 1.6 + st.z * 40.0));
                let tint = if st.z > 0.85 { 0xff_ff55 } else { 0xff_ffff };
                self.put(f, 0.0, st.x * 0.45, yy, ids.star, Opt { s: 0.35 + st.z * 0.4, alpha: twinkle, tint, ..D });
            }
        }
        self.put(f, 0.0, -6.5, 4.4 - oy * 0.3, ids.moon, Opt { s: 1.5, ..D });
        for x in -22..22 {
            p!(x as f32 + 0.5, g + 0.9, ids.hill_top, Opt { tint: 0x1b_1638, ..D });
        }
        p!(0.0, g + 0.2, fl, Opt { sx: 40.0, sy: 0.42, tint: 0x1b_1638, ..D });
        p!(-8.2, g + 1.75, ids.house, Opt { s: 1.4, tint: 0x8a_90bb, ..D });
        p!(-9.1, g + 1.3, ids.puff, Opt { s: 1.8, alpha: 0.2, tint: 0xff_d866, ..D });
        for i in 0..44 {
            p!(i as f32 - 21.5, g + 0.5, ids.fence, Opt { tint: 0x5e_6490, ..D });
        }
        p!(6.0, g + 3.75, ids.big_tree, Opt { s: 1.5, tint: 0x9a_a2cc, ..D });
        p!(5.16, g + 4.5, ids.puff, Opt { s: 2.0, alpha: 0.2, tint: 0xff_d866, ..D });
        if !launch {
            p!(6.0, g + 0.45, ids.puff, Opt { s: 1.3 + 0.15 * sin(t * 3.0), alpha: 0.4, tint: 0xff_cc44, ..D });
            p!(6.0, g + 1.4, fl, Opt { sx: 0.5, sy: 1.8, alpha: 0.1, tint: 0xff_dd88, ..D });
        }
        macro_rules! saucer {
            () => {
                p!(0.0, self.ly, ids.saucer, Opt { s: 1.6, rot: sin(t * 6.0) * 0.02, ..D })
            };
        }
        if launch && self.ly < -3.2 {
            saucer!();
        }
        p!(0.0, g - 4.0, fl, Opt { sx: 40.0, sy: 8.0, tint: 0x0b_2412, ..D });
        p!(0.0, g - 0.06, fl, Opt { sx: 40.0, sy: 0.14, tint: 0x2f_7a34, ..D });
        let k = if launch { (big_t / 1.2).min(1.0) } else { 0.0 };
        if k > 0.0 {
            p!(0.0, g - 0.08, fl, Opt { sx: 2.6 * k, sy: 0.2, tint: 0x00_0000, ..D });
            p!(0.0, g + 0.15, ids.puff, Opt { s: 2.2 * k, alpha: 0.35 * k, tint: 0x55_ffff, ..D });
        }
        p!(-0.65 - k * 1.3, g - 0.04, fl, Opt { sx: 1.3, sy: 0.14, tint: 0x88_88aa, ..D });
        p!(0.65 + k * 1.3, g - 0.04, fl, Opt { sx: 1.3, sy: 0.14, tint: 0x88_88aa, ..D });
        if launch && self.ly >= -3.2 {
            saucer!();
        }
        if !launch {
            let bx = -10.0 + big_t.min(6.6) * 2.2;
            let id = if big_t < 6.6 { ids.run[((t * 8.0) as i32 & 1) as usize ^ 1] } else { ids.stand };
            p!(bx, g + 0.75, id, Opt { tint: 0xc8_ccee, ..D });
        }
        for i in 0..9 {
            let fx = hashf(i, 9) * 22.0 - 11.0 + sin(t * 0.6 + i as f32) * 1.2;
            let fy = g + 0.6 + hashf(i, 4) * 3.0 + sin(t * 1.3 + i as f32 * 2.0) * 0.4;
            let glow = 0.25 + 0.75 * sin(t * 2.4 + i as f32 * 1.7).max(0.0);
            p!(fx, fy, ids.star, Opt { s: 0.6, alpha: glow, tint: 0xff_ff55, ..D });
        }
    }

    fn lab(&self, f: &mut [u8]) {
        let (t, big_t, g) = (self.t, self.st_t, GROUND);
        let ids = &self.ids;
        let fl = ids.fill;
        let blink = (t * 3.0) as i32 & 1;
        macro_rules! p {
            ($x:expr, $y:expr, $id:expr, $o:expr) => {
                self.put(f, 0.0, $x, $y, $id, $o)
            };
        }
        for x in -22..22 {
            for y in (floor(g) as i32)..8 {
                p!(x as f32 + 0.5, y as f32 + 0.5, ids.lab_panel, Opt { tint: 0x5a_5a88, ..D });
            }
        }
        p!(0.0, 5.6, fl, Opt { sx: 40.0, sy: 0.3, tint: 0x3a_3a55, ..D });
        p!(0.0, 5.38, fl, Opt { sx: 40.0, sy: 0.1, tint: 0x8a_8ab0, ..D });
        p!(-4.2, 1.6, ids.blueprint, Opt { s: 1.3, ..D });
        p!(-1.1, 2.1, ids.blueprint, Opt { flip: true, ..D });
        p!(6.0, 5.1, fl, Opt { sx: 4.6, sy: 0.35, tint: 0x0c_0c18, ..D });
        p!(6.0, 4.88, fl, Opt { sx: 4.6, sy: 0.08, tint: 0x55_ffff, alpha: 0.6 + 0.4 * blink as f32, ..D });
        p!(-11.3, g + 1.0, ids.console[blink as usize], D);
        p!(11.3, g + 1.0, ids.console[1 - blink as usize], Opt { flip: true, ..D });
        p!(6.0, g + 0.3, ids.pad, Opt { s: 1.2, ..D });
        p!(6.0, g + 1.4 + sin(t * 2.0) * 0.05, ids.saucer, Opt { s: 1.6, ..D });
        p!(-1.2, 4.4, ids.lamp, D);
        for y in (floor(g) as i32)..8 {
            p!(-8.0, y as f32 + 0.5, ids.ladder, D);
        }
        p!(0.0, g - 4.0, fl, Opt { sx: 40.0, sy: 8.0, tint: 0x22_223a, ..D });
        p!(0.0, g - 0.06, fl, Opt { sx: 40.0, sy: 0.14, tint: 0x9a_9ac0, ..D });
        p!(-1.0, g + 0.5, ids.bench, D);
        p!(-1.22, g + 0.78, ids.puff, Opt { s: 0.9 + 0.15 * sin(t * 4.0), alpha: 0.35, tint: 0xff_ff88, ..D });
        let (mut bx, mut by, mut walk) = (-8.0, g + 0.75, false);
        if big_t < 2.2 {
            by = 4.0 + (g + 0.75 - 4.0) * (big_t / 2.2);
        } else {
            let ww = (big_t - 2.2).min(2.35);
            bx = -8.0 + ww * 2.3;
            walk = ww < 2.35;
        }
        let id = if walk { ids.run[((t * 8.0) as i32 & 1) as usize ^ 1] } else { ids.stand };
        p!(bx, by, id, D);
    }
}
