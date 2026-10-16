// Writes the visible world as sprite instances and selects the frame lights.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Scene drawing: walks the visible region of the world and writes sprite instances into the
//! shared buffer (painter's order), plus the per-frame light selection.

#[allow(unused_imports)]
use alloc::{boxed::Box, vec, vec::Vec};
#[allow(unused_imports)]
use lf_sim::fmath::{FloatMath, FloatMath32};
use crate::ents::*;
use crate::sprites::*;
use crate::tiles::*;
use crate::world::{Mode, Player, World, MANTLE_RISE_SHARE};
use lf_sim::{hashf, PushOpts};

pub mod flags {
    pub const NIGHT: u32 = 1;
    pub const CULLING: u32 = 2;
    pub const STRESS: u32 = 4;
}

/// Indices into the `out` array the shell reads each frame.
pub mod out {
    pub const COUNT: usize = 0;
    pub const CAM_X: usize = 1;
    pub const CAM_Y: usize = 2;
    pub const LIGHTS: usize = 3;
    pub const WORLD: usize = 4;
    pub const CLEAR_R: usize = 5;
    pub const CLEAR_G: usize = 6;
    pub const CLEAR_B: usize = 7;
    pub const CLEAR_A: usize = 8;
    pub const AMB_R: usize = 9;
    pub const AMB_G: usize = 10;
    pub const AMB_B: usize = 11;
    pub const LIGHTING: usize = 12;
    pub const SKY_GLOW: usize = 13;
    /// The direction to the sun in sprite space (x right, y up, z out of the screen), then its colour. A
    /// colour of zero is no sun, and is what levels, the title and the cinematic write.
    pub const SUN_X: usize = 14;
    pub const SUN_Y: usize = 15;
    pub const SUN_Z: usize = 16;
    pub const SUN_R: usize = 17;
    pub const SUN_G: usize = 18;
    pub const SUN_B: usize = 19;
    /// How many floats the shell reads; the rest are for later.
    pub const LEN: usize = 32;
}

pub enum Layer {
    Stars,
    Hills {
        s: Spr,
        tint: u32,
        nt: u32,
        f: f64,
        base: f64,
        amp: f64,
    },
    Wall {
        s: u16,
        tint: u32,
        nt: u32,
        f: f64,
    },
    /// Drifting clouds scattered on a coarse grid; `base` and `spread` set their altitude band.
    Clouds {
        s: Spr,
        tint: u32,
        nt: u32,
        f: f64,
        base: f64,
        spread: f64,
        speed: f64,
        alpha: f32,
    },
}

/// How a level plays and looks; chosen per level, independent of the overworld area it sits in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Theme {
    Crater,
    Caves,
    Citadel,
    OpenSky,
    Building,
    Theatre,
    Shaft,
    Foundry,
}

/// How the camera follows Ben.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CamMode {
    /// Look ahead sideways, with a modest vertical chase.
    Side,
    /// Tall levels: no look-ahead and a fast vertical chase.
    Tower,
}

impl Theme {
    pub const fn cam(self) -> CamMode {
        match self {
            Theme::Building | Theme::Shaft => CamMode::Tower,
            _ => CamMode::Side,
        }
    }

    /// Index of this theme's tile set for `tileset_tile`.
    pub const fn tiles(self) -> u8 {
        match self {
            Theme::Crater => 0,
            Theme::Caves | Theme::Shaft => 1,
            Theme::Citadel => 2,
            Theme::OpenSky => 3,
            Theme::Building => 4,
            Theme::Theatre => 5,
            Theme::Foundry => 6,
        }
    }
}

/// Lighting, parallax and sprites for one level theme.
pub struct LevelTheme {
    pub clear: u32,
    pub amb: [f32; 3],
    pub night: [f32; 3],
    pub lm: f64,
    pub lantern: bool,
    pub layers: Vec<Layer>,
    pub crys: Spr,
    pub lc: [f64; 3],
}

pub fn theme(t: Theme) -> LevelTheme {
    match t {
        Theme::Caves | Theme::Shaft => LevelTheme {
            clear: 0x000000,
            amb: [0.85, 0.85, 0.95],
            night: [0.14, 0.12, 0.24],
            lm: 0.6,
            lantern: true,
            layers: vec![Layer::Wall {
                s: tileset_tile(Theme::Caves.tiles(), BT_BACK),
                tint: 0x50506a,
                nt: 0x9a9a9a,
                f: 0.5,
            }],
            crys: Spr::CrysC,
            lc: [0.35, 1.1, 1.2],
        },
        Theme::Citadel => LevelTheme {
            clear: 0x000000,
            amb: [0.95, 0.9, 0.95],
            night: [0.3, 0.24, 0.32],
            lm: 0.6,
            lantern: true,
            layers: vec![Layer::Wall {
                s: tileset_tile(Theme::Citadel.tiles(), BT_BACK),
                tint: 0x8a4a70,
                nt: 0x8a6a7a,
                f: 0.6,
            }],
            crys: Spr::CrysM,
            lc: [1.2, 0.5, 1.1],
        },
        Theme::OpenSky => LevelTheme {
            clear: 0x55ffff,
            amb: [1.0, 1.0, 1.0],
            night: [0.34, 0.34, 0.5],
            lm: 0.55,
            lantern: true,
            layers: vec![
                Layer::Stars,
                Layer::Clouds {
                    s: Spr::Cloud,
                    tint: 0xffffff,
                    nt: 0x3a3a68,
                    f: 0.08,
                    base: 6.5,
                    spread: 10.0,
                    speed: 0.25,
                    alpha: 0.9,
                },
                Layer::Hills {
                    s: Spr::MtnTop,
                    tint: 0xaa5500,
                    nt: 0x2a1a30,
                    f: 0.2,
                    base: 3.5,
                    amp: 2.5,
                },
                Layer::Hills {
                    s: Spr::HillTop,
                    tint: 0x55ff55,
                    nt: 0x14202a,
                    f: 0.45,
                    base: 2.5,
                    amp: 1.5,
                },
                Layer::Clouds {
                    s: Spr::Cloud,
                    tint: 0xffffff,
                    nt: 0x4a4a78,
                    f: 0.6,
                    base: 5.0,
                    spread: 5.0,
                    speed: 0.6,
                    alpha: 0.75,
                },
            ],
            crys: Spr::CrysM,
            lc: [1.0, 0.85, 0.5],
        },
        Theme::Foundry => LevelTheme {
            clear: 0x100808,
            amb: [0.85, 0.72, 0.66],
            night: [0.34, 0.24, 0.24],
            lm: 0.6,
            lantern: true,
            layers: vec![Layer::Wall {
                s: tileset_tile(Theme::Foundry.tiles(), BT_BACK),
                tint: 0xa8806e,
                nt: 0x5a4038,
                f: 0.6,
            }],
            crys: Spr::CrysM,
            lc: [1.4, 0.7, 0.3],
        },
        Theme::Theatre => LevelTheme {
            clear: 0x200010,
            amb: [0.62, 0.5, 0.58],
            night: [0.3, 0.22, 0.3],
            lm: 0.6,
            lantern: true,
            layers: vec![Layer::Wall {
                s: tileset_tile(Theme::Theatre.tiles(), BT_BACK),
                tint: 0xbb9aaa,
                nt: 0x6a4a5a,
                f: 0.7,
            }],
            crys: Spr::CrysM,
            lc: [1.2, 0.8, 0.5],
        },
        Theme::Building => LevelTheme {
            clear: 0x55ffff,
            amb: [1.0, 0.97, 0.9],
            night: [0.3, 0.28, 0.4],
            lm: 0.6,
            lantern: true,
            layers: vec![
                Layer::Stars,
                Layer::Clouds {
                    s: Spr::Cloud,
                    tint: 0xffffff,
                    nt: 0x3a3a68,
                    f: 0.1,
                    base: 78.0,
                    spread: 8.0,
                    speed: 0.25,
                    alpha: 0.9,
                },
            ],
            crys: Spr::CrysM,
            lc: [1.0, 0.8, 0.45],
        },
        Theme::Crater => LevelTheme {
            clear: 0x5555ff,
            amb: [1.0, 1.0, 1.0],
            night: [0.32, 0.28, 0.42],
            lm: 0.55,
            lantern: true,
            layers: vec![
                Layer::Stars,
                Layer::Hills {
                    s: Spr::MtnTop,
                    tint: 0xaa00aa,
                    nt: 0x3a1858,
                    f: 0.22,
                    base: 4.5,
                    amp: 2.0,
                },
                Layer::Hills {
                    s: Spr::HillTop,
                    tint: 0x00aa00,
                    nt: 0x14102a,
                    f: 0.5,
                    base: 3.5,
                    amp: 1.5,
                },
            ],
            crys: Spr::CrysM,
            lc: [1.0, 0.67, 0.3],
        },
    }
}

fn js_round(x: f64) -> f64 {
    (x + 0.5).floor()
}

fn rgb(c: u32) -> [f32; 3] {
    [
        ((c >> 16) & 255) as f32 / 255.0,
        ((c >> 8) & 255) as f32 / 255.0,
        (c & 255) as f32 / 255.0,
    ]
}

impl World {
    pub fn push(&mut self, x: f64, y: f64, id: impl Into<u16>, o: &PushOpts) {
        let sp = self.spr[id.into() as usize];
        self.inst.push(x, y, &sp, o);
    }

    fn sprite_h(&self, id: u16) -> f64 {
        f64::from(self.spr[id as usize].h)
    }

    /// Fills the instance buffer for the current scene. `alpha` is the interpolation factor
    /// between the previous and current tick.
    pub fn render(&mut self, alpha: f64, fl: u32) {
        self.inst.clear();
        self.lights.clear();
        let night = fl & flags::NIGHT != 0;
        let sh = if self.shake > 0.0 {
            (self.t * 70.0).sin() * self.shake * 0.15
        } else {
            0.0
        };
        let cx = self.pcx + (self.cam_x - self.pcx) * alpha + sh;
        let cy = self.pcy + (self.cam_y - self.pcy) * alpha;
        self.out[out::CAM_X] = cx as f32;
        self.out[out::CAM_Y] = cy as f32;
        let bio =
            (self.mode == Mode::Level || self.mode == Mode::Attract).then(|| theme(self.theme));
        let sky_glow = night
            && bio
                .as_ref()
                .is_some_and(|b| matches!(b.layers.first(), Some(Layer::Stars)));
        let clear = match &bio {
            _ if sky_glow => (0, 0.0),
            Some(b) if !night => (b.clear, 1.0),
            _ => (0, 1.0),
        };
        let c = rgb(clear.0);
        self.out[out::CLEAR_R] = c[0];
        self.out[out::CLEAR_G] = c[1];
        self.out[out::CLEAR_B] = c[2];
        self.out[out::CLEAR_A] = clear.1 as f32;
        self.out[out::SKY_GLOW] = f32::from(u8::from(sky_glow));
        // The overworld is lit by day only: Options' night switch does not reach it.
        let map_view = self.mode == Mode::Map;
        let amb = if map_view {
            self.map_amb
        } else {
            bio.as_ref()
                .map_or([1.0; 3], |b| if night { b.night } else { b.amb })
        };
        self.out[out::AMB_R] = amb[0];
        self.out[out::AMB_G] = amb[1];
        self.out[out::AMB_B] = amb[2];
        self.out[out::LIGHTING] = f32::from(u8::from(bio.is_some() || map_view));
        self.out[out::SUN_X..=out::SUN_B].fill(0.0);
        if map_view {
            // The same sun over every area, its colour the rest of full brightness: flat ground stays itself.
            let d = map_sun_dir();
            self.out[out::SUN_X] = d[0];
            self.out[out::SUN_Y] = d[1];
            self.out[out::SUN_Z] = d[2];
            for (i, a) in amb.iter().enumerate() {
                self.out[out::SUN_R + i] = ((1.0 - a) / d[2]).max(0.0);
            }
        }
        self.out[out::WORLD] = 0.0;
        let view = [
            cx - self.half_w - 1.0,
            cx + self.half_w + 1.0,
            cy - self.half_h - 1.0,
            cy + self.half_h + 1.0,
        ];
        let mut world = 0usize;
        match self.mode {
            Mode::Map => world += self.draw_map(alpha, view, fl, (cx, cy)),
            Mode::Level | Mode::Attract => {
                if let Some(b) = &bio {
                    world += self.draw_level(alpha, view, fl, (cx, cy), b);
                }
            }
            Mode::None => {}
        }
        let nl = match &bio {
            Some(b) => self.update_lights(b, night, (cx, cy)),
            None if map_view => self.update_map_lights((cx, cy)),
            None => 0,
        };
        self.out[out::COUNT] = self.inst.len() as f32;
        self.out[out::LIGHTS] = nl as f32;
        self.out[out::WORLD] = world as f32;
    }

    /// Pushes a sprite standing on the tile `(tx, ty)`: centred on the tile across, its base on the tile's
    /// bottom edge, so a taller sprite (a tree, a house) rises above the tile into the one north of it.
    fn push_base(&mut self, tx: f64, ty: f64, id: Spr, o: &PushOpts) {
        let h = self.sprite_h(id as u16);
        self.push(tx + 0.5, ty + h / 32.0, id, o);
    }

    /// Whether tile `(x, y)` is land: on the map and not water.
    fn is_land(&self, x: i32, y: i32) -> bool {
        self.map.in_bounds(x, y) && self.map.get(x, y) != RIVER
    }

    /// The edge of a water tile where land meets it: a short brown bank face under land to the north (the
    /// land's south side, which stands up), and a thin dirt lip where land lies to the south, west or east.
    fn draw_shore(&mut self, x: i32, y: i32, o: &PushOpts) {
        if self.is_land(x, y + 1) {
            self.push(
                f64::from(x) + 0.5,
                f64::from(y) + 1.0 - 3.0 / 16.0,
                Spr::OwBank,
                o,
            );
        }
        self.draw_lips(
            x,
            y,
            [
                false,
                self.is_land(x, y - 1),
                self.is_land(x - 1, y),
                self.is_land(x + 1, y),
            ],
            o,
        );
    }

    /// A thin dirt lip along each side of a tile that is flagged: `[north, south, west, east]`.
    fn draw_lips(&mut self, x: i32, y: i32, sides: [bool; 4], o: &PushOpts) {
        let (fx, fy) = (f64::from(x) + 0.5, f64::from(y) + 0.5);
        let (tx, ty) = (f64::from(x), f64::from(y));
        if sides[0] {
            self.push(fx, ty + 1.0 - 1.0 / 16.0, Spr::OwLipH, o);
        }
        if sides[1] {
            self.push(
                fx,
                ty + 1.0 / 16.0,
                Spr::OwLipH,
                &PushOpts { sy: -1.0, ..*o },
            );
        }
        if sides[2] {
            self.push(tx + 1.0 / 16.0, fy, Spr::OwLipV, o);
        }
        if sides[3] {
            self.push(
                tx + 1.0 - 1.0 / 16.0,
                fy,
                Spr::OwLipV,
                &PushOpts { flip: true, ..*o },
            );
        }
    }

    /// A dirt lip along each side of a path tile that is not path, so the road has an edge.
    fn draw_path_edges(&mut self, x: i32, y: i32, o: &PushOpts) {
        let not_path = |w: &World, dx: i32, dy: i32| w.map.get(x + dx, y + dy) != PATH;
        let sides = [
            not_path(self, 0, 1),
            not_path(self, 0, -1),
            not_path(self, -1, 0),
            not_path(self, 1, 0),
        ];
        self.draw_lips(x, y, sides, o);
    }

    fn draw_map(&mut self, alpha: f64, v: [f64; 4], fl: u32, _cam: (f64, f64)) -> usize {
        let cul = fl & flags::CULLING != 0;
        let (w, h) = (self.map.w, self.map.h);
        let (tx0, tx1, ty0, ty1) = if cul {
            (
                v[0].floor().max(0.0) as i32,
                v[1].ceil().min(f64::from(w - 1)) as i32,
                v[2].floor().max(0.0) as i32,
                v[3].ceil().min(f64::from(h - 1)) as i32,
            )
        } else {
            (0, w - 1, 0, h - 1)
        };
        let rf = ((self.t * 2.0).floor() as i64) & 1;
        // The map is lit now. The ground, the scenery and the icons take the sun and the lights; Ben and the
        // paths keep a minimum brightness so the way always reads; teleporters and flags glow of their own.
        let lit = PushOpts::default();
        let act = PushOpts { actor: true, ..lit };
        let glow = PushOpts::em();
        let shade = PushOpts {
            alpha: 0.55,
            ..glow
        };

        // Pass 1: the ground, flat, in any order.
        for y in ty0..=ty1 {
            for x in tx0..=tx1 {
                let tt = self.map.get(x, y);
                let (fx, fy) = (f64::from(x) + 0.5, f64::from(y) + 0.5);
                let theme = crate::levels::area_theme(self.map_areas[(y * w + x) as usize]);
                if tt == RIVER {
                    let s = if (i64::from(x) + i64::from(y) + rf) & 1 != 0 {
                        theme.river[0]
                    } else {
                        theme.river[1]
                    };
                    self.push(
                        fx,
                        fy,
                        s,
                        &PushOpts {
                            tint: theme.river_tint,
                            ..lit
                        },
                    );
                    self.draw_shore(x, y, &lit);
                    // A glint now and then, each tile on its own slow beat.
                    if (hashf(x + 91, y + 53) + self.t * 0.35).fract() < 0.1 {
                        self.push(fx, fy, Spr::OwGlint, &glow);
                    }
                    continue;
                }
                if tt == PATH {
                    self.push(fx, fy, Spr::OwPath, &act);
                    self.draw_path_edges(x, y, &act);
                } else {
                    let look = (hashf(x + 31, y + 17) * 3.0) as usize % 3;
                    self.push(fx, fy, theme.grass[look], &lit);
                    // A flower, pebbles or a tuft on about one plain tile in six, a little off centre.
                    if tt == GRASS && hashf(x * 3 + 1, y * 7 + 2) < 0.17 {
                        let kind = (hashf(x + 5, y + 9) * 4.0) as usize % 4;
                        let jx = (hashf(x + 2, y + 6) - 0.5) * 0.5;
                        let jy = (hashf(x + 8, y + 3) - 0.5) * 0.5;
                        let decal =
                            [Spr::OwDecal0, Spr::OwDecal1, Spr::OwDecal2, Spr::OwDecal3][kind];
                        self.push(fx + jx, fy + jy, decal, &lit);
                    }
                }
            }
        }

        // Pass 2: everything that stands on the ground, from the north row down to the south one, so that what
        // is further south (nearer the viewer) is drawn over what is behind it. A tall sprite rises above its
        // tile, so rows up to two below the view still reach into it.
        let p = &self.p;
        let px = p.b.px + (p.b.x - p.b.px) * alpha;
        let py = p.b.py + (p.b.y - p.b.py) * alpha;
        // The row of his centre, which is what collision and the area's light go by, so he is never drawn in a
        // row behind the one he is standing in.
        let ben_row = (py + 0.3).floor() as i32;
        let first = (ty0 - 2).max(0);
        // One row past the north edge too: a landmark is drawn in the row above its node.
        for y in (first..=ty1 + 1).rev() {
            for x in tx0..=tx1 {
                if y >= h {
                    break;
                }
                let tt = self.map.get(x, y);
                let tile = (f64::from(x), f64::from(y));
                let theme = crate::levels::area_theme(self.map_areas[(y * w + x) as usize]);
                match tt {
                    TREE => {
                        self.push_base(tile.0, tile.1 - 0.05, Spr::OwShadowS, &shade);
                        let tree = if hashf(x, y) < 0.5 {
                            theme.trees[0]
                        } else {
                            theme.trees[1]
                        };
                        self.push_base(tile.0, tile.1, tree, &lit);
                    }
                    ROCK => {
                        self.push_base(tile.0, tile.1 - 0.05, Spr::OwShadowS, &shade);
                        self.push_base(tile.0, tile.1, theme.rock, &lit);
                    }
                    HOUSE_A | HOUSE_B | SHOP => {
                        let s = match tt {
                            HOUSE_A => Spr::OwHouseA,
                            HOUSE_B => Spr::OwHouseB,
                            _ => Spr::OwShop,
                        };
                        self.push_base(tile.0, tile.1 - 0.05, Spr::OwShadowL, &shade);
                        self.push_base(tile.0, tile.1, s, &lit);
                    }
                    FOUNTAIN => self.push_base(tile.0, tile.1, Spr::OwFountain, &lit),
                    _ => {}
                }
            }
            // Each level's landmark, drawn in the row above its node: a person standing in that row, on the
            // path that leads down to the node, is drawn after it and so in front of it.
            for i in 0..self.points.len() {
                let pt = self.points[i];
                if pt.kind != crate::levels::PtKind::Level || pt.y.floor() as i32 + 1 != y {
                    continue;
                }
                let icon = crate::levels::LEVELS[usize::from(pt.level)].icon;
                if pt.big {
                    self.push(pt.x + 1.0, pt.y + 1.0, icon, &lit);
                } else {
                    // Standing a little way up its node's tile, so Ben at the doorstep is below it.
                    self.push(pt.x + 0.5, pt.y + 0.35 + 1.0, icon, &lit);
                }
            }
            for i in 0..self.points.len() {
                let pt = self.points[i];
                if pt.y.floor() as i32 != y || (pt.hidden && !self.met(pt.req)) {
                    continue;
                }
                let (x, y) = (pt.x + 0.5, pt.y + 0.5);
                match pt.kind {
                    crate::levels::PtKind::Saucer => self.push(x, y, Spr::Saucer, &lit),
                    crate::levels::PtKind::Sign => self.push(x, y, Spr::OwSign, &lit),
                    crate::levels::PtKind::Tele => {
                        let on = self.met(pt.req);
                        let s = if on && ((self.t * 4.0).floor() as i64) & 1 != 0 {
                            Spr::OwTele1
                        } else {
                            Spr::OwTele0
                        };
                        self.push(
                            x,
                            y,
                            s,
                            &PushOpts {
                                tint: if on { 0xffffff } else { 0x777777 },
                                ..glow
                            },
                        );
                    }
                    crate::levels::PtKind::Level => {
                        // Every level has a pole that holds only a flag: bare until the level is cleared, then the
                        // flag is up. A level cleared just now has its flag thrown from Ben to the pole.
                        let (fx, fy) = (x + 0.6, y + 0.9);
                        let o = PushOpts { scale: 0.7, ..glow };
                        let flying = self.flag_throw.filter(|&(l, _)| l == pt.level);
                        if let Some((_, t)) = flying {
                            let k = (t / crate::world::FLAG_THROW_SECS).clamp(0.0, 1.0);
                            let (bx, by) = (self.p.b.x + 0.3, self.p.b.y + 0.4);
                            let lift = 4.0 * k * (1.0 - k) * 1.6;
                            self.push(fx, fy, Spr::OwFlagHolder, &o);
                            self.push(
                                bx + (fx - bx) * k,
                                by + (fy - by) * k + lift,
                                Spr::OwFlag,
                                &o,
                            );
                        } else if self.game.is_done(pt.level) {
                            self.push(fx, fy, Spr::OwFlag, &o);
                        } else {
                            self.push(fx, fy, Spr::OwFlagHolder, &o);
                        }
                    }
                }
            }
            if y == ben_row {
                let p = &self.p;
                let s = if (p.anim.floor() as i64) & 1 != 0 {
                    Spr::BenMap1
                } else {
                    Spr::BenMap0
                };
                let flip = p.face < 0.0;
                self.push_base(px + 0.3 - 0.5, py - 0.05, Spr::OwShadowS, &shade);
                self.push(px + 0.3, py + 0.4, s, &PushOpts { flip, ..act });
            }
        }
        (w * h) as usize
    }

    fn draw_level(
        &mut self,
        alpha: f64,
        v: [f64; 4],
        fl: u32,
        cam: (f64, f64),
        bio: &LevelTheme,
    ) -> usize {
        let cul = fl & flags::CULLING != 0;
        let night = fl & flags::NIGHT != 0;
        let (x0, x1, y0, y1) = (v[0], v[1], v[2], v[3]);
        let (cx, cy) = cam;
        let t = self.t;
        let em = PushOpts::em();
        let mut world = 0usize;
        for layer in &bio.layers {
            match layer {
                Layer::Stars => {
                    if !night {
                        continue;
                    }
                    let f = 0.04;
                    let (ox, oy, c) = (cx * (1.0 - f), cy * (1.0 - f), 2.5);
                    let (gx0, gx1) = (
                        ((x0 - ox) / c).floor() as i32,
                        ((x1 - ox) / c).ceil() as i32,
                    );
                    let (gy0, gy1) = (
                        ((y0 - oy) / c).floor() as i32,
                        ((y1 - oy) / c).ceil() as i32,
                    );
                    for gx in gx0..=gx1 {
                        for gy in gy0..=gy1 {
                            let hh = hashf(gx, gy);
                            if hh > 0.3 {
                                continue;
                            }
                            let tint = if hh < 0.05 {
                                0xffff55
                            } else if hh < 0.1 {
                                0x55ffff
                            } else {
                                0xffffff
                            };
                            self.push(
                                (f64::from(gx) + hashf(gy, gx)) * c + ox,
                                (f64::from(gy) + hashf(gx + 7, gy + 3)) * c + oy,
                                Spr::Star,
                                &PushOpts {
                                    scale: 0.9,
                                    alpha: (0.6 + 0.4 * (t * 1.5 + hh * 60.0).sin()) as f32,
                                    tint,
                                    ..em
                                },
                            );
                        }
                    }
                }
                Layer::Hills {
                    s,
                    tint,
                    nt,
                    f,
                    base,
                    amp,
                } => {
                    let (ox, oy) = (cx * (1.0 - f), cy * (1.0 - f) * 0.6);
                    let tn = if night { *nt } else { *tint };
                    for ix in ((x0 - ox).floor() as i32)..=((x1 - ox).ceil() as i32) {
                        let fi = f64::from(ix);
                        let ht = js_round(
                            base + amp * (fi * 0.19 + f * 9.0).sin()
                                + 2.0 * (fi * 0.067 + f * 20.0).sin(),
                        ) + oy;
                        let wx = fi + 0.5 + ox;
                        self.push(wx, ht + 0.5, *s, &PushOpts { tint: tn, ..em });
                        if ht > y0 {
                            self.push(
                                wx,
                                (ht + y0) / 2.0,
                                Spr::Fill,
                                &PushOpts {
                                    sy: (ht - y0) as f32,
                                    tint: tn,
                                    ..em
                                },
                            );
                        }
                    }
                }
                Layer::Clouds {
                    s,
                    tint,
                    nt,
                    f,
                    base,
                    spread,
                    speed,
                    alpha,
                } => {
                    let (ox, oy) = (cx * (1.0 - f), cy * (1.0 - f) * 0.6);
                    let drift = t * speed;
                    let cell = 8.0;
                    let o = PushOpts {
                        tint: if night { *nt } else { *tint },
                        alpha: *alpha,
                        ..em
                    };
                    let (g0, g1) = (
                        ((x0 - ox - drift) / cell).floor() as i32 - 1,
                        ((x1 - ox - drift) / cell).ceil() as i32,
                    );
                    for g in g0..=g1 {
                        if hashf(g, 11) < 0.35 {
                            continue;
                        }
                        let wx = (f64::from(g) + hashf(g, 23)) * cell + drift + ox;
                        let wy = base + spread * hashf(g, 37) + oy;
                        self.push(
                            wx,
                            wy,
                            *s,
                            &PushOpts {
                                scale: (0.8 + 0.6 * hashf(g, 5)) as f32,
                                ..o
                            },
                        );
                    }
                }
                Layer::Wall { s, tint, nt, f } => {
                    let (ox, oy) = (cx * (1.0 - f), cy * (1.0 - f));
                    let o = if night {
                        PushOpts::tint(*nt)
                    } else {
                        PushOpts { tint: *tint, ..em }
                    };
                    for ix in ((x0 - ox).floor() as i32)..=((x1 - ox).ceil() as i32) {
                        for iy in ((y0 - oy).floor() as i32)..=((y1 - oy).ceil() as i32) {
                            self.push(f64::from(ix) + 0.5 + ox, f64::from(iy) + 0.5 + oy, *s, &o);
                        }
                    }
                }
            }
        }
        if fl & flags::STRESS != 0 {
            let (bx0, bx1) = if cul {
                (x0.floor().max(-100.0) as i32, x1.ceil().min(299.0) as i32)
            } else {
                (-100, 299)
            };
            let (by0, by1) = if cul {
                (y0.floor().max(-60.0) as i32, y1.ceil().min(64.0) as i32)
            } else {
                (-60, 64)
            };
            let sp = tileset_tile(self.theme.tiles(), BT_BACK);
            for x in bx0..=bx1 {
                for y in by0..=by1 {
                    self.push(
                        f64::from(x) + 0.5,
                        f64::from(y) + 0.5,
                        sp,
                        &PushOpts::tint(0x707070),
                    );
                }
            }
            world += 50_000;
        }

        // Tiles
        let (w, h) = (self.map.w, self.map.h);
        let fr = ((t * 3.0).floor() as i64) & 1;
        let (tx0, tx1, ty0, ty1) = if cul {
            (
                x0.floor().max(0.0) as i32,
                x1.ceil().min(f64::from(w - 1)) as i32,
                y0.floor().max(0.0) as i32,
                y1.ceil().min(f64::from(h - 1)) as i32,
            )
        } else {
            (0, w - 1, 0, h - 1)
        };
        // A mural is drawn from its bottom-left tile but spans three columns and two rows, so scan
        // a little beyond the view or its edge pops out while most of it is still on screen.
        let (tx0, ty0) = ((tx0 - 2).max(0), (ty0 - 1).max(0));
        let b = self.theme.tiles();
        for y in ty0..=ty1 {
            for x in tx0..=tx1 {
                let tt = self.map.get(x, y);
                if tt == 0 {
                    continue;
                }
                let up = if y + 1 < h {
                    self.map.get(x, y + 1)
                } else {
                    FILL
                };
                let mut op = PushOpts::default();
                if self.theme == Theme::Building
                    && (tt == RUNG || tt == RUNG_TOP || tt == CRYS || self.map.is_slope(tt))
                {
                    // These tiles have see-through corners and sit against the interior wall.
                    self.push(
                        f64::from(x) + 0.5,
                        f64::from(y) + 0.5,
                        tileset_tile(b, BT_BACK),
                        &op,
                    );
                }
                let sp: u16 = match tt {
                    FILL => {
                        if up == FILL
                            || up == BLOCK
                            || self.map.is_slope(up)
                            || up == DOOR_R
                            || up == DOOR_B
                            || up == DOOR_G
                        {
                            tileset_tile(b, BT_FILL)
                        } else {
                            tileset_tile(b, BT_TOP)
                        }
                    }
                    BLOCK => tileset_tile(b, BT_BLOCK),
                    PLAT => tileset_tile(b, BT_PLAT),
                    WALLBG => tileset_tile(b, BT_BACK),
                    MURAL => {
                        // One instance draws the whole 3-by-2 picture from its bottom-left tile.
                        self.push(
                            f64::from(x) + 1.5,
                            f64::from(y) + 1.0,
                            Spr::Mural,
                            &PushOpts::default(),
                        );
                        continue;
                    }
                    RUNG => Spr::Ladder as u16,
                    RUNG_TOP => Spr::LadderTop as u16,
                    VINE => Spr::Vine as u16,
                    CRACKED => Spr::Cracked as u16,
                    SPIKE => Spr::SpikeTile as u16,
                    CONV_L => {
                        if fr != 0 {
                            Spr::ConveyL1 as u16
                        } else {
                            Spr::ConveyL0 as u16
                        }
                    }
                    CONV_R => {
                        if fr != 0 {
                            Spr::ConveyR1 as u16
                        } else {
                            Spr::ConveyR0 as u16
                        }
                    }
                    FURNACE => {
                        op.emissive = true;
                        match (up == FURNACE, fr != 0) {
                            (true, true) => Spr::FurnDeep1 as u16,
                            (true, false) => Spr::FurnDeep0 as u16,
                            (false, true) => Spr::FurnTop1 as u16,
                            (false, false) => Spr::FurnTop0 as u16,
                        }
                    }
                    CHOC => {
                        op.emissive = true;
                        match (up == CHOC, fr != 0) {
                            (true, true) => Spr::ChocDeep1 as u16,
                            (true, false) => Spr::ChocDeep0 as u16,
                            (false, true) => Spr::ChocTop1 as u16,
                            (false, false) => Spr::ChocTop0 as u16,
                        }
                    }
                    DOOR_R => Spr::DoorRed as u16,
                    DOOR_B => Spr::DoorBlue as u16,
                    DOOR_G => Spr::DoorGreen as u16,
                    GATE | GATE_2 | GATE_3 => {
                        let on = self.map.switch(self.map.props.channel(tt));
                        op.alpha = if on { 1.0 } else { 0.18 };
                        Spr::Gate as u16
                    }
                    BRIDGE => {
                        op.alpha = if self.map.switch(0) { 1.0 } else { 0.22 };
                        Spr::Bridge as u16
                    }
                    CRYS => {
                        op.emissive = true;
                        bio.crys as u16
                    }
                    EXIT => {
                        op.emissive = true;
                        if up == EXIT {
                            Spr::ExitBot as u16
                        } else {
                            Spr::ExitTop as u16
                        }
                    }
                    R45..=L22B => tileset_tile(b, BT_SLOPE0 + u16::from(tt - R45)),
                    _ => continue,
                };
                self.push(f64::from(x) + 0.5, f64::from(y) + 0.5, sp, &op);
            }
        }
        world += self.solid_count;

        let vis =
            |x: f64, y: f64, r: f64| !cul || (x > x0 - r && x < x1 + r && y > y0 - r && y < y1 + r);
        for i in 0..self.plats.len() {
            let pl = &self.plats[i];
            if vis(pl.x, pl.y, 2.0) {
                let (x, y) = (pl.x + pl.w / 2.0, pl.y + pl.h / 2.0);
                let lift = pl.w > 2.5;
                self.push(
                    x,
                    y,
                    match (lift, fr != 0) {
                        (false, false) => Spr::Hover0,
                        (false, true) => Spr::Hover1,
                        (true, false) => Spr::Lift0,
                        (true, true) => Spr::Lift1,
                    },
                    &PushOpts::default(),
                );
            }
        }
        for i in 0..self.items.len() {
            let it = self.items[i];
            if it.taken || !vis(it.x, it.y, 2.0) {
                continue;
            }
            let s = match it.kind {
                ItemKind::Cheezie => Spr::Cheezie,
                ItemKind::Choc => Spr::Choc,
                ItemKind::Cookie => Spr::Cookie,
                ItemKind::Soda => Spr::Soda,
                ItemKind::KeyRed => Spr::KeyRed,
                ItemKind::KeyBlue => Spr::KeyBlue,
                ItemKind::KeyGreen => Spr::KeyGreen,
                ItemKind::Usb => Spr::Usb,
            };
            let scale = if it.kind == ItemKind::Usb { 1.2 } else { 1.0 };
            self.push(
                it.x,
                it.y + (t * 3.0 + it.x).sin() * 0.08,
                s,
                &PushOpts { scale, ..em },
            );
        }
        for i in 0..self.ents.len() {
            let e = &self.ents[i];
            if e.dead {
                continue;
            }
            let b = &e.b;
            let (x, y) = (b.px + (b.x - b.px) * alpha, b.py + (b.y - b.py) * alpha);
            if !vis(x, y, 3.0) {
                continue;
            }
            let f2 = ((e.t * 4.0).floor() as i64) & 1 != 0;
            let mut op = PushOpts {
                flip: e.dir > 0.0,
                tint: if e.stun > 0.0 { 0xaaaaaa } else { 0xffffff },
                ..Default::default()
            };
            let bx = x + b.w / 2.0;
            let sp = match e.kind {
                Kind::Gloop => {
                    if f2 {
                        Spr::Gloop1
                    } else {
                        Spr::Gloop0
                    }
                }
                Kind::Hopper => {
                    if b.on_ground {
                        Spr::Hopper0
                    } else {
                        Spr::Hopper1
                    }
                }
                Kind::Marsh => {
                    if b.vy > 4.0 || !b.on_ground {
                        Spr::Marsh0
                    } else {
                        Spr::Marsh1
                    }
                }
                Kind::Beetle => {
                    let a = if e.state == St::Charge {
                        ((e.t * 12.0).floor() as i64) & 1 != 0
                    } else {
                        f2
                    };
                    if a {
                        Spr::Beetle1
                    } else {
                        Spr::Beetle0
                    }
                }
                Kind::Bat => {
                    if e.state == St::Idle && e.stun <= 0.0
                        || ((e.t * 10.0).floor() as i64) & 1 == 0
                    {
                        Spr::Bat0
                    } else {
                        Spr::Bat1
                    }
                }
                Kind::Pod => {
                    if e.puff {
                        Spr::Pod1
                    } else {
                        Spr::Pod0
                    }
                }
                Kind::Phantom => {
                    op.alpha = e.alpha.max(0.2) as f32;
                    if e.alpha < 0.6 {
                        Spr::Phantom1
                    } else {
                        Spr::Phantom0
                    }
                }
                Kind::Roller => {
                    op.rot = e.rot as f32;
                    op.flip = false;
                    Spr::Roller
                }
                Kind::Sentry => {
                    if f2 {
                        Spr::Sentry1
                    } else {
                        Spr::Sentry0
                    }
                }
                Kind::Drone => {
                    op.flip = false;
                    if ((e.t * 8.0).floor() as i64) & 1 != 0 {
                        Spr::Drone1
                    } else {
                        Spr::Drone0
                    }
                }
                Kind::Boss => {
                    op.flip = e.dir > 0.0;
                    if e.state == St::Hot && ((e.t * 8.0).floor() as i64) & 1 != 0 {
                        op.tint = 0xffaaaa;
                    }
                    if e.state == St::Hot || e.state == St::Down {
                        Spr::Boss1
                    } else {
                        Spr::Boss0
                    }
                }
                Kind::Press => {
                    op = PushOpts::default();
                    if e.b.y < e.ay - crate::world::PRESS_DANGER_DROP {
                        Spr::Press1
                    } else {
                        Spr::Press0
                    }
                }
                Kind::Mirror | Kind::Swivel => {
                    op = em;
                    op.flip = e.dir < 0.0;
                    Spr::Mirror
                }
                Kind::CrystalSwitch => {
                    op = em;
                    if self.map.switch(e.dir as u8) {
                        Spr::CrysSwitchOff
                    } else {
                        Spr::CrysSwitchOn
                    }
                }
                Kind::Switch => {
                    op = PushOpts::default();
                    if self.map.switch(0) {
                        Spr::SwitchOn
                    } else {
                        Spr::SwitchOff
                    }
                }
                Kind::Target => {
                    op = em;
                    Spr::Target
                }
                Kind::Cameo => {
                    op = PushOpts::default();
                    if e.dir > 0.0 {
                        Spr::BillyAlt
                    } else {
                        Spr::Billy
                    }
                }
                Kind::Glyph => {
                    op = PushOpts::default();
                    Spr::Glyph
                }
                Kind::Terminal => {
                    op = em;
                    if ((e.t * 2.0).floor() as i64) & 1 != 0 {
                        Spr::Terminal1
                    } else {
                        Spr::Terminal0
                    }
                }
                Kind::Cage => {
                    op = PushOpts::default();
                    if self.hacked {
                        Spr::Billy
                    } else {
                        Spr::BillyCage
                    }
                }
            };
            if !op.emissive {
                op.actor = true;
            }
            let (stun, puff, stunned_t, ew, ey, eh, et) =
                (e.stun > 0.0, e.puff, e.t, b.w, b.y, b.h, e.t);
            let is_pod = e.kind == Kind::Pod;
            let ph = self.sprite_h(sp as u16);
            self.push(bx, y + ph / 32.0 - 1.0 / 16.0, sp, &op);
            if stun {
                let s = if ((stunned_t * 6.0).floor() as i64) & 1 != 0 {
                    Spr::Stars1
                } else {
                    Spr::Stars0
                };
                self.push(bx, y + eh + 0.3, s, &em);
            }
            if is_pod && puff && !stun {
                let k = et % 1.0;
                let _ = ey;
                for (i, ox) in [-1.2, -2.0, ew + 1.2, ew + 2.0].into_iter().enumerate() {
                    let sx = x + if ox < 0.0 { ox + 0.4 } else { ox - 0.4 };
                    self.push(
                        sx,
                        y + 0.6 + k * 0.4,
                        if i & 1 != 0 { Spr::Spore1 } else { Spr::Spore0 },
                        &PushOpts { alpha: 0.9, ..em },
                    );
                }
            }
        }
        world += self.items.len() + self.ents.len() + self.plats.len();
        for i in 0..self.shots.len() {
            let sh = self.shots[i];
            self.push(sh.x, sh.y, sh.sprite, &em);
        }
        for i in 0..self.fx.len() {
            let f = self.fx[i];
            self.push(
                f.x,
                f.y,
                Spr::Puff,
                &PushOpts {
                    scale: (0.5 + f.t * 1.5) as f32,
                    alpha: (1.0 - f.t / f.life) as f32,
                    tint: f.tint,
                    ..em
                },
            );
        }
        let p = &self.p;
        if !p.hidden {
            let px = p.b.px + (p.b.x - p.b.px) * alpha;
            let py = p.b.py + (p.b.y - p.b.py) * alpha;
            let blink = p.inv > 0.0 && ((t * 12.0).floor() as i64) & 1 != 0;
            if !blink {
                let sp = ben_sprite(p);
                let (flip, rot) = (p.face < 0.0, if p.dead > 0.0 { p.rot as f32 } else { 0.0 });
                let ph = self.sprite_h(sp as u16);
                self.push(
                    px + 0.35,
                    py + ph / 32.0,
                    sp,
                    &PushOpts {
                        flip,
                        rot,
                        actor: true,
                        ..Default::default()
                    },
                );
            }
        }
        // Painted flats go over everything, so a hidden room hides what is inside until it fades.
        if !self.rooms.is_empty() {
            for y in ty0..=ty1 {
                for x in tx0..=tx1 {
                    let room = self.room_at(x, y);
                    let t = self.map.get(x, y);
                    // The flat also covers a mural hung in the room: it is a tile, not an empty
                    // cell, so no flat was laid there, yet it must stay hidden with the rest.
                    if t != FACADE && !(room.is_some() && (t == MURAL || t == MURAL_PART)) {
                        continue;
                    }
                    let a = room.map_or(1.0, |i| self.room_alpha[i]);
                    if a < 0.02 {
                        continue;
                    }
                    self.push(
                        f64::from(x) + 0.5,
                        f64::from(y) + 0.5,
                        Spr::Facade,
                        &PushOpts {
                            alpha: a as f32,
                            ..Default::default()
                        },
                    );
                    world += 1;
                }
            }
        }
        world
    }

    /// The overworld's lights, each pulsing at its own rate, picked nearest the camera first by day.
    fn update_map_lights(&mut self, cam: (f64, f64)) -> usize {
        let t = self.t;
        for i in 0..self.map_lights.len() {
            let l = self.map_lights[i];
            let k = if l.pulse > 0.0 {
                0.85 + 0.15 * (t * core::f64::consts::TAU * l.pulse).sin()
            } else {
                1.0
            };
            self.lights.add(
                cam,
                l.x,
                l.y,
                l.z,
                l.r,
                [l.c[0] * k, l.c[1] * k, l.c[2] * k],
            );
        }
        self.lights
            .select(MAP_LIGHT_MUL, &mut self.lights_pos, &mut self.lights_col)
    }

    fn update_lights(&mut self, bio: &LevelTheme, night: bool, cam: (f64, f64)) -> usize {
        let t = self.t;
        if bio.lantern && !self.p.hidden {
            let p = &self.p.b;
            if let Some(i) =
                self.lights
                    .add(cam, p.x + p.w / 2.0, p.y + 1.1, 1.2, 5.5, [0.95, 0.7, 0.45])
            {
                self.lights.pin_first(i);
            }
        }
        for l in &self.static_lights {
            let fl = 1.6 + 0.12 * (t * 9.0 + l.x).sin() + 0.08 * (t * 23.0 + l.x * 3.0).sin();
            self.lights.add(
                cam,
                l.x,
                l.y + 0.4,
                1.4,
                8.5,
                [l.c[0] * fl, l.c[1] * fl, l.c[2] * fl],
            );
        }
        for h in &self.hazards {
            let k = 0.6 + 0.4 * (t * 4.0 + h.x).sin();
            self.lights.add(
                cam,
                h.x,
                h.y,
                0.9,
                3.0 + h.w * 0.4,
                [0.96 * k * 1.3, 0.25 * k, 0.37 * k],
            );
        }
        for sh in &self.shots {
            let c = if sh.ben {
                [0.3, 1.1, 1.2]
            } else {
                [1.2, 0.3, 0.3]
            };
            self.lights.add(cam, sh.x, sh.y, 0.6, 3.0, c);
        }
        for e in &self.ents {
            if e.dead {
                continue;
            }
            let b = &e.b;
            match e.kind {
                Kind::Sentry => {
                    self.lights
                        .add(cam, b.x + b.w / 2.0, b.y + 0.4, 0.8, 3.5, [1.3, 0.2, 0.2]);
                }
                Kind::Pod if e.puff => {
                    self.lights
                        .add(cam, b.x + 0.4, b.y + 0.8, 0.8, 4.0, [1.2, 0.3, 1.2]);
                }
                Kind::Terminal => {
                    self.lights
                        .add(cam, b.x + 0.5, b.y + 1.2, 1.0, 4.0, [0.3, 1.2, 0.3]);
                }
                Kind::Boss if e.state == St::Hot => {
                    self.lights
                        .add(cam, b.x + b.w / 2.0, b.y + 2.4, 1.2, 6.0, [1.4, 1.2, 0.3]);
                }
                _ => {}
            }
        }
        for it in &self.items {
            if it.taken {
                continue;
            }
            let c = match it.kind {
                ItemKind::Cheezie => [0.9, 0.6, 0.15],
                ItemKind::Choc => [0.7, 0.4, 0.2],
                ItemKind::Cookie => [0.9, 0.7, 0.3],
                ItemKind::Soda => [0.15, 0.75, 0.9],
                ItemKind::KeyRed => [1.1, 0.3, 0.3],
                ItemKind::KeyBlue => [0.3, 0.4, 1.2],
                ItemKind::KeyGreen => [0.3, 1.1, 0.4],
                ItemKind::Usb => [1.2, 1.1, 0.4],
            };
            let r = if matches!(
                it.kind,
                ItemKind::Usb | ItemKind::KeyRed | ItemKind::KeyBlue | ItemKind::KeyGreen
            ) {
                3.2
            } else {
                2.4
            };
            self.lights.add(cam, it.x, it.y, 0.8, r, c);
        }
        let mul = if night { 1.0 } else { bio.lm };
        self.lights
            .select(mul, &mut self.lights_pos, &mut self.lights_col)
    }
}

/// How strong the overworld's point lights are by day, against 1.0 at night in a level.
pub(crate) const MAP_LIGHT_MUL: f64 = 0.35;

/// The direction to the overworld's sun in sprite space (x right, y up, z out of the screen): from the
/// west-north-west and fairly high, so a sprite's relief catches light on its left and top.
pub(crate) fn map_sun_dir() -> [f32; 3] {
    let (x, y, z) = (-0.5f32, 0.3f32, 0.81f32);
    let len = (x * x + y * y + z * z).sqrt();
    [x / len, y / len, z / len]
}

/// Picks Ben's sprite from his current state.
pub(crate) fn ben_sprite(p: &Player) -> Spr {
    if p.dead > 0.0 {
        Spr::BenJump
    } else if let Some(m) = &p.mantle {
        // Grip the ledge, then pull up with the arms over the lip, then push up onto the top.
        let u = m.progress();
        if u < MANTLE_RISE_SHARE / 2.0 {
            Spr::BenMantle1
        } else if u < MANTLE_RISE_SHARE {
            Spr::BenMantle2
        } else {
            Spr::BenMantle3
        }
    } else if p.pogo {
        if p.squash > 0.0 {
            Spr::BenPogo2
        } else {
            Spr::BenPogo
        }
    } else if p.climb && !(p.shoot_t > 0.0 && p.aim != 0) {
        // Firing up or down along a ladder shows the aim pose instead of the climb frames.
        if ((p.anim * 1.6).floor() as i64) & 1 != 0 {
            Spr::BenClimb2
        } else {
            Spr::BenClimb1
        }
    } else if p.shoot_t > 0.0 {
        match p.aim {
            1 => Spr::BenAimUp,
            -1 => Spr::BenAimDown,
            _ => Spr::BenShoot,
        }
    } else if !p.b.on_ground {
        Spr::BenJump
    } else if p.b.vx.abs() > 0.5 {
        if ((p.anim * 1.4).floor() as i64) & 1 != 0 {
            Spr::BenRun2
        } else {
            Spr::BenRun1
        }
    } else if p.look_up > 0.0 {
        Spr::BenLookUp
    } else if p.look_down > 0.0 {
        Spr::BenLookDown
    } else {
        Spr::BenStand
    }
}
