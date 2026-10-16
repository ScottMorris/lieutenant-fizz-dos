// Builders for the Episode 1 levels and overworld tile maps and spawns.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Level and overworld builders: a port of the prototype's `levels.js`.

#[allow(unused_imports)]
use alloc::{boxed::Box, vec, vec::Vec};
#[allow(unused_imports)]
use lf_sim::fmath::{FloatMath, FloatMath32};
use crate::ents::{Item, ItemKind, Kind, Spawn};
use crate::render::Theme;
use crate::sprites::Spr;
use crate::tiles::*;
use lf_sim::{hashf, Platform, TileMap};

pub const CRATER: u8 = 0;
pub const CAVES: u8 = 1;
pub const CITADEL: u8 = 2;
pub const METEOR_MESA: u8 = 3;
pub const ZARG_LOOKOUT: u8 = 4;
pub const MARSHMALLOW_MEADOWS: u8 = 5;
pub const BONBON_PLAYHOUSE: u8 = 6;
pub const FUDGE_BOG: u8 = 7;
pub const MIRROR_SHAFTS: u8 = 8;
pub const SUGAR_GLASS_GALLERY: u8 = 9;
pub const FROSTING_FLATS: u8 = 10;
pub const FROSTING_SPIRE: u8 = 11;
pub const COCOA_FOUNDRY: u8 = 12;
pub const GUMDROP_ISLE: u8 = 13;
pub const WHISPER_HOLLOW: u8 = 14;

/// Number of levels; level ids are `0..LEVEL_COUNT` and double as bit positions in `Game::done`.
pub const LEVEL_COUNT: u8 = 15;
/// Ben's saucer, a walk-through with no goal: it has an id and a table entry but no cleared bit.
pub const SAUCER: u8 = 15;
/// Every id the level table knows: the cleared-level ids plus the saucer.
pub const TABLE_LEN: u8 = 16;

/// Overworld areas, in the order `MapData::areas` and `area_theme` index them.
pub const AREA_CRATER_FIELDS: u8 = 0;
pub const AREA_MARSHMALLOW_MEADOWS: u8 = 1;
pub const AREA_ROCK_CANDY_REACH: u8 = 2;
pub const AREA_FROSTING_FRONTIER: u8 = 3;
/// The island in the central lake, home to the secret level.
pub const AREA_GUMDROP_ISLE: u8 = 4;

/// Static description of one level: how it is built and where it sits on the overworld.
pub struct LevelDef {
    pub id: u8,
    pub build: fn() -> LevelData,
    pub theme: Theme,
    pub area: u8,
    /// Overworld marker sprite.
    pub icon: Spr,
}

/// Every level, indexed by id.
pub const LEVELS: [LevelDef; TABLE_LEN as usize] = [
    LevelDef {
        id: CRATER,
        build: crater,
        theme: Theme::Crater,
        area: AREA_CRATER_FIELDS,
        icon: Spr::OwLmCrater,
    },
    LevelDef {
        id: CAVES,
        build: caves,
        theme: Theme::Caves,
        area: AREA_ROCK_CANDY_REACH,
        icon: Spr::OwLmCaves,
    },
    LevelDef {
        id: CITADEL,
        build: citadel,
        theme: Theme::Citadel,
        area: AREA_FROSTING_FRONTIER,
        icon: Spr::OwCastle,
    },
    LevelDef {
        id: METEOR_MESA,
        build: meteor_mesa,
        theme: Theme::OpenSky,
        area: AREA_CRATER_FIELDS,
        icon: Spr::OwLmMesa,
    },
    LevelDef {
        id: ZARG_LOOKOUT,
        build: zarg_lookout,
        theme: Theme::Building,
        area: AREA_CRATER_FIELDS,
        icon: Spr::OwLmLookout,
    },
    LevelDef {
        id: MARSHMALLOW_MEADOWS,
        build: marshmallow_meadows,
        theme: Theme::OpenSky,
        area: AREA_MARSHMALLOW_MEADOWS,
        icon: Spr::OwLmMeadows,
    },
    LevelDef {
        id: BONBON_PLAYHOUSE,
        build: bonbon_playhouse,
        theme: Theme::Theatre,
        area: AREA_MARSHMALLOW_MEADOWS,
        icon: Spr::OwLmPlayhouse,
    },
    LevelDef {
        id: FUDGE_BOG,
        build: fudge_bog,
        theme: Theme::Crater,
        area: AREA_MARSHMALLOW_MEADOWS,
        icon: Spr::OwLmFudgeBog,
    },
    LevelDef {
        id: MIRROR_SHAFTS,
        build: mirror_shafts,
        theme: Theme::Shaft,
        area: AREA_ROCK_CANDY_REACH,
        icon: Spr::OwLmShafts,
    },
    LevelDef {
        id: SUGAR_GLASS_GALLERY,
        build: sugar_glass_gallery,
        theme: Theme::Theatre,
        area: AREA_ROCK_CANDY_REACH,
        icon: Spr::OwLmSugarGlass,
    },
    LevelDef {
        id: FROSTING_FLATS,
        build: frosting_flats,
        theme: Theme::OpenSky,
        area: AREA_FROSTING_FRONTIER,
        icon: Spr::OwLmFlats,
    },
    LevelDef {
        id: FROSTING_SPIRE,
        build: frosting_spire,
        theme: Theme::Building,
        area: AREA_FROSTING_FRONTIER,
        icon: Spr::OwLmSpire,
    },
    LevelDef {
        id: COCOA_FOUNDRY,
        build: cocoa_foundry,
        theme: Theme::Foundry,
        area: AREA_FROSTING_FRONTIER,
        icon: Spr::OwLmFoundry,
    },
    LevelDef {
        id: GUMDROP_ISLE,
        build: gumdrop_isle,
        theme: Theme::OpenSky,
        area: AREA_GUMDROP_ISLE,
        icon: Spr::OwLmGumdrop,
    },
    LevelDef {
        id: WHISPER_HOLLOW,
        build: whisper_hollow,
        theme: Theme::Caves,
        area: AREA_ROCK_CANDY_REACH,
        icon: Spr::OwLmHollow,
    },
    LevelDef {
        id: SAUCER,
        build: saucer,
        theme: Theme::Building,
        area: AREA_CRATER_FIELDS,
        icon: Spr::Saucer,
    },
];

/// Bit in `Game::done` for a cleared level.
/// `id` must be a real level id: anything from 32 up would overflow the shift.
pub const fn level_bit(id: u8) -> u32 {
    debug_assert!(id < LEVEL_COUNT);
    1 << id
}
#[derive(Clone, Copy, Debug)]
pub struct Arena {
    pub x0: f64,
    pub x1: f64,
    pub floor: f64,
}

/// A rectangle of tiles (inclusive) hidden behind `FACADE` tiles until Ben steps inside.
#[derive(Clone, Copy, Debug)]
pub struct Room {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
    /// Walking into it sets the secret-found bit (once), which reveals the hidden teleporter.
    pub secret: bool,
}

impl Room {
    pub fn overlaps(&self, x: f64, y: f64, w: f64, h: f64) -> bool {
        x < f64::from(self.x1 + 1)
            && x + w > f64::from(self.x0)
            && y < f64::from(self.y1 + 1)
            && y + h > f64::from(self.y0)
    }

    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x0 && x <= self.x1 && y >= self.y0 && y <= self.y1
    }
}

pub struct LevelData {
    pub id: u8,
    pub theme: Theme,
    pub area: u8,
    pub map: TileMap,
    pub spawns: Vec<Spawn>,
    pub items: Vec<Item>,
    pub plats: Vec<Platform>,
    pub start: (f64, f64),
    pub arena: Option<Arena>,
    pub rooms: Vec<Room>,
}

/// A 32-bit FNV-1a hash, built up byte by byte; used to fingerprint a level.
struct Fnv(u32);

impl Fnv {
    fn new() -> Self {
        Fnv(0x811c_9dc5)
    }

    fn byte(&mut self, b: u8) {
        self.0 = (self.0 ^ u32::from(b)).wrapping_mul(0x0100_0193);
    }

    fn u32(&mut self, v: u32) {
        v.to_le_bytes().into_iter().for_each(|b| self.byte(b));
    }

    fn f64(&mut self, v: f64) {
        v.to_bits()
            .to_le_bytes()
            .into_iter()
            .for_each(|b| self.byte(b));
    }
}

impl LevelData {
    /// A fingerprint of what the level is made of: its tiles and what is placed in it (spawns, items,
    /// platforms, the start, the arena and the hidden rooms). It changes when a level is edited, so a
    /// recording can tell its level is no longer the one it was made on. It does not cover the rules
    /// (physics, enemy behaviour), which a build stamp covers, nor the theme, which only changes looks.
    pub fn fingerprint(&self) -> u32 {
        let mut h = Fnv::new();
        h.byte(self.id);
        h.byte(self.area);
        h.u32(self.map.w as u32);
        h.u32(self.map.h as u32);
        self.map.data.iter().for_each(|&b| h.byte(b));
        h.u32(self.map.switches);
        h.byte(u8::from(self.map.floor_solid));
        for s in &self.spawns {
            h.byte(s.kind as u8);
            h.f64(s.x);
            h.f64(s.y);
            h.f64(s.dir);
            h.byte(u8::from(s.ride));
        }
        for i in &self.items {
            h.byte(i.kind as u8);
            h.f64(i.x);
            h.f64(i.y);
        }
        for p in &self.plats {
            for v in [
                p.x, p.y, p.w, p.h, p.ax, p.ay, p.bx, p.by, p.speed, p.ph, p.dwell,
            ] {
                h.f64(v);
            }
        }
        h.f64(self.start.0);
        h.f64(self.start.1);
        if let Some(a) = &self.arena {
            h.byte(1);
            h.f64(a.x0);
            h.f64(a.x1);
            h.f64(a.floor);
        }
        for r in &self.rooms {
            for v in [r.x0, r.y0, r.x1, r.y1] {
                h.u32(v as u32);
            }
            h.byte(u8::from(r.secret));
        }
        h.0
    }
}

enum Seg {
    Flat,
    Up,
    Down,
    Up22,
    Down22,
    Gap,
    Pool,
    Spikes,
    /// A conveyor belt carrying Ben to the right.
    BeltR,
    /// A conveyor belt carrying Ben to the left.
    BeltL,
}
use Seg::*;

struct Builder {
    w: i32,
    h: i32,
    map: TileMap,
    h_at: Vec<f64>,
    spawns: Vec<Spawn>,
    items: Vec<Item>,
    plats: Vec<Platform>,
    rooms: Vec<Room>,
    x: i32,
    ground: i32,
    /// The liquid `Pool` segments are filled with.
    liquid: u8,
}

impl Builder {
    fn new(w: i32, h: i32, ground: i32) -> Self {
        Builder {
            w,
            h,
            map: TileMap::new(w, h, props()),
            h_at: vec![-1.0; w as usize],
            spawns: vec![],
            items: vec![],
            plats: vec![],
            rooms: vec![],
            x: 0,
            ground,
            liquid: CHOC,
        }
    }

    fn fill(&mut self, x0: i32, x1: i32, y0: i32, y1: i32, t: u8) -> &mut Self {
        for x in x0..=x1 {
            for y in y0..=y1 {
                self.map.set(x, y, t);
            }
        }
        self
    }

    fn col(&mut self, x: i32, h: i32) {
        self.fill(x, x, 0, h - 1, FILL);
        if x >= 0 && x < self.w {
            self.h_at[x as usize] = f64::from(h);
        }
    }

    fn set_h(&mut self, x: i32, v: f64) {
        if x >= 0 && x < self.w {
            self.h_at[x as usize] = v;
        }
    }

    fn run(&mut self, segs: &[(Seg, i32)]) -> &mut Self {
        for (k, n) in segs {
            for _ in 0..*n {
                let x = self.x;
                match k {
                    Flat => {
                        self.col(x, self.ground);
                        self.x += 1;
                    }
                    Up => {
                        self.col(x, self.ground);
                        self.map.set(x, self.ground, R45);
                        self.set_h(x, f64::from(self.ground) + 0.5);
                        self.x += 1;
                        self.ground += 1;
                    }
                    Down => {
                        self.ground -= 1;
                        self.col(x, self.ground);
                        self.map.set(x, self.ground, L45);
                        self.set_h(x, f64::from(self.ground) + 0.5);
                        self.x += 1;
                    }
                    Up22 => {
                        self.col(x, self.ground);
                        self.map.set(x, self.ground, R22A);
                        self.col(x + 1, self.ground);
                        self.map.set(x + 1, self.ground, R22B);
                        self.x += 2;
                        self.ground += 1;
                    }
                    Down22 => {
                        self.ground -= 1;
                        self.col(x, self.ground);
                        self.map.set(x, self.ground, L22A);
                        self.col(x + 1, self.ground);
                        self.map.set(x + 1, self.ground, L22B);
                        self.x += 2;
                    }
                    Gap => self.x += 1,
                    Pool => {
                        self.map.set(x, 0, FILL);
                        let liquid = self.liquid;
                        self.fill(x, x, 1, self.ground - 2, liquid);
                        self.set_h(x, f64::from(self.ground) - 1.0);
                        self.x += 1;
                    }
                    Spikes => {
                        self.col(x, self.ground);
                        self.map.set(x, self.ground, SPIKE);
                        self.x += 1;
                    }
                    BeltR | BeltL => {
                        self.col(x, self.ground);
                        let belt = if matches!(k, BeltR) { CONV_R } else { CONV_L };
                        self.map.set(x, self.ground - 1, belt);
                        self.x += 1;
                    }
                }
            }
        }
        self
    }

    fn ground_at(&self, x: f64) -> f64 {
        let i = x.floor() as usize;
        self.h_at.get(i).copied().unwrap_or(-1.0)
    }

    fn ent(&mut self, kind: Kind, x: f64) -> &mut Self {
        let y = self.ground_at(x);
        self.spawns.push(Spawn {
            kind,
            x,
            y,
            dir: -1.0,
            ride: false,
        });
        self
    }

    fn ent_at(&mut self, kind: Kind, x: f64, y: f64) -> &mut Self {
        self.spawns.push(Spawn {
            kind,
            x,
            y,
            dir: -1.0,
            ride: false,
        });
        self
    }

    /// An enemy that rides moving platforms (most enemies pass through them).
    fn rider(&mut self, kind: Kind, x: f64, y: f64) -> &mut Self {
        self.spawns.push(Spawn {
            kind,
            x,
            y,
            dir: -1.0,
            ride: true,
        });
        self
    }

    /// A spawn with an explicit orientation, such as a `/` mirror (`dir` = 1).
    fn ent_dir(&mut self, kind: Kind, x: f64, y: f64, dir: f64) -> &mut Self {
        self.spawns.push(Spawn {
            kind,
            x,
            y,
            dir,
            ride: false,
        });
        self
    }

    fn item(&mut self, kind: ItemKind, x: i32, y: i32) -> &mut Self {
        self.items.push(Item {
            kind,
            x: f64::from(x) + 0.5,
            y: f64::from(y) + 0.5,
            taken: false,
        });
        self
    }

    fn row(&mut self, kind: ItemKind, x0: i32, n: i32, y: i32, step: i32) -> &mut Self {
        for i in 0..n {
            self.item(kind, x0 + i * step, y);
        }
        self
    }

    fn plat(&mut self, x0: i32, x1: i32, y: i32) -> &mut Self {
        self.fill(x0, x1, y, y, PLAT)
    }

    fn hover(&mut self, x: f64, y: f64, bx: f64, by: f64, speed: f64) -> &mut Self {
        self.plats.push(Platform::new(x, y, bx, by, speed));
        self
    }

    /// Hides a rectangle behind painted flats: every empty cell in it becomes `FACADE`, so floors,
    /// ledges and ladders inside stay solid and visible once the flat fades.
    fn room(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) -> &mut Self {
        for x in x0..=x1 {
            for y in y0..=y1 {
                if self.map.get(x, y) == EMPTY {
                    self.map.set(x, y, FACADE);
                }
            }
        }
        self.rooms.push(Room {
            x0,
            y0,
            x1,
            y1,
            secret: false,
        });
        self
    }

    /// A hidden room that counts as the secret: stepping into it is how the player finds the clue.
    fn secret_room(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) -> &mut Self {
        self.room(x0, y0, x1, y1);
        if let Some(r) = self.rooms.last_mut() {
            r.secret = true;
        }
        self
    }

    /// A 3-by-2 mural whose bottom-left tile is (x, y).
    fn mural(&mut self, x: i32, y: i32) -> &mut Self {
        self.fill(x, x + 2, y, y + 1, MURAL_PART);
        self.map.set(x, y, MURAL);
        self
    }

    /// A hidden room reached by a vine: a vine climbs from `foot` to a rock shelf at row `shelf`
    /// (Ben stands on it at that height), a two-tile pocket runs away from the vine in direction
    /// `dir`, and a cracked wall as tall as the room closes it off from a room `depth` tiles deep and
    /// `h` tall. Carved out of whatever solid rock is already there, so the caller fills the mass
    /// first and leaves at least a row of rock above `shelf + h`.
    fn vine_nook(
        &mut self,
        vx: i32,
        foot: i32,
        shelf: i32,
        dir: i32,
        depth: i32,
        h: i32,
    ) -> &mut Self {
        let far = vx + dir * (3 + depth);
        let (x0, x1) = (vx.min(far), vx.max(far));
        self.fill(x0, x1, shelf - 2, shelf - 1, FILL);
        self.fill(x0, x1, shelf, shelf + h - 1, EMPTY);
        let cx = vx + dir * 3;
        self.fill(cx, cx, shelf, shelf + h - 1, CRACKED);
        self.fill(vx, vx, foot, shelf + 1, VINE)
    }

    /// A snack cache on a small ledge `rise` tiles above the ground at `x0`: a pogo bounce with
    /// Jump held lifts Ben about six and a half tiles (enough to knock the cookie loose), an ordinary
    /// jump only about three and a half.
    fn pogo_cache(&mut self, x0: i32, x1: i32, rise: i32) -> &mut Self {
        let stand = self.ground_at(f64::from(x0)) as i32 + rise;
        self.plat(x0, x1, stand - 1)
            .item(Cookie, x0 + 1, stand)
            .item(Cheezie, x0, stand)
            .item(Cheezie, x1, stand)
    }

    /// A ladder from standing row `y0` up to the ledge at row `y1`: rungs below, a standable top.
    fn ladder(&mut self, x: i32, y0: i32, y1: i32) -> &mut Self {
        self.fill(x, x, y0, y1 - 1, RUNG);
        self.map.set(x, y1, RUNG_TOP);
        self
    }

    /// `n` columns of stairs rising to the right from standing row `y`; the last column's slope
    /// tile sits on row `y + n - 1`, so the walkable floor above begins at `y + n`.
    fn stairs_right(&mut self, x0: i32, y: i32, n: i32) -> &mut Self {
        for i in 0..n {
            self.fill(x0 + i, x0 + i, y, y + i - 1, FILL);
            self.map.set(x0 + i, y + i, R45);
        }
        self
    }

    /// The mirror image: stairs rising to the left.
    fn stairs_left(&mut self, x0: i32, y: i32, n: i32) -> &mut Self {
        for i in 0..n {
            let h = n - 1 - i;
            self.fill(x0 + i, x0 + i, y, y + h - 1, FILL);
            self.map.set(x0 + i, y + h, L45);
        }
        self
    }

    /// Fills every empty cell in the rectangle with interior wall.
    fn wall_behind(&mut self, x0: i32, x1: i32, y0: i32, y1: i32) -> &mut Self {
        for x in x0..=x1 {
            for y in y0..=y1 {
                if self.map.get(x, y) == EMPTY {
                    self.map.set(x, y, WALLBG);
                }
            }
        }
        self
    }

    fn crys(&mut self, x: i32) -> &mut Self {
        let y = self.ground_at(f64::from(x)).ceil() as i32;
        self.map.set(x, y, CRYS);
        self
    }

    fn walls(&mut self) -> &mut Self {
        let (w, h) = (self.w, self.h);
        self.fill(0, 0, 0, h - 1, BLOCK);
        self.fill(w - 1, w - 1, 0, h - 1, BLOCK);
        self
    }

    fn out(self, id: u8, start: (f64, f64), arena: Option<Arena>) -> LevelData {
        let def = &LEVELS[id as usize];
        LevelData {
            id,
            theme: def.theme,
            area: def.area,
            map: self.map,
            spawns: self.spawns,
            items: self.items,
            plats: self.plats,
            start,
            arena,
            rooms: self.rooms,
        }
    }
}

use ItemKind::*;

fn crater() -> LevelData {
    let mut b = Builder::new(192, 28, 4);
    b.run(&[
        (Flat, 14),
        (Up, 2),
        (Flat, 6),
        (Down, 2),
        (Flat, 5),
        (Pool, 4),
        (Flat, 8),
        (Up22, 2),
        (Flat, 10),
        (Gap, 3),
        (Flat, 9),
        (Down22, 2),
        (Flat, 6),
        (Spikes, 4),
        (Flat, 10),
        (Up, 3),
        (Flat, 8),
        (Down, 3),
        (Pool, 6),
        (Flat, 10),
        (Up22, 3),
        (Flat, 14),
        (Down, 3),
        (Flat, 12),
        (Spikes, 3),
        (Flat, 8),
        (Up, 2),
        (Flat, 22),
    ]);
    b.walls();
    b.row(Cheezie, 17, 4, 7, 1)
        .row(Cheezie, 29, 4, 7, 1)
        .item(Soda, 36, 5)
        .row(Choc, 46, 3, 8, 3);
    b.fill(49, 51, 10, 10, BLOCK).item(Cookie, 50, 11);
    b.plat(107, 108, 5).row(Cheezie, 105, 6, 8, 1);
    b.row(Cheezie, 72, 3, 5, 1)
        .row(Choc, 94, 4, 8, 2)
        .item(Soda, 125, 9)
        .row(Cheezie, 128, 5, 8, 2);
    b.plat(131, 134, 12)
        .item(KeyRed, 132, 13)
        .row(Choc, 146, 3, 5, 3)
        .item(Cookie, 157, 7);
    b.fill(171, 171, 6, 7, DOOR_R).fill(171, 171, 8, 27, BLOCK);
    b.fill(186, 186, 6, 7, EXIT).row(Cheezie, 175, 4, 7, 2);
    for x in [10, 26, 60, 98, 118, 150, 178] {
        b.crys(x);
    }
    b.ent(Kind::Gloop, 35.0)
        .ent(Kind::Gloop, 62.0)
        .ent(Kind::Gloop, 85.0);
    b.ent(Kind::Hopper, 97.0)
        .ent(Kind::Hopper, 148.0)
        .ent(Kind::Marsh, 115.0)
        .ent(Kind::Pod, 136.0);
    b.ent_at(Kind::Drone, 88.5, 7.5)
        .ent_at(Kind::Drone, 165.5, 7.5);
    b.pogo_cache(8, 10, 7);
    b.out(CRATER, (3.0, 4.0), None)
}

/// Open-sky daylight level: biscuit-rock mesas separated by gaps that cloud ledges and a hover
/// platform bridge. Gaps are at most 3 wide unless a ledge or hover platform carries Ben across: Ben
/// only clears 4 tiles by jumping within a tenth of a second of the edge.
fn meteor_mesa() -> LevelData {
    let mut b = Builder::new(200, 30, 4);
    b.run(&[
        (Flat, 12),
        (Up, 2),
        (Flat, 6),
        (Gap, 3),
        (Flat, 9),
        (Up, 3),
        (Flat, 6),
        (Down, 2),
        (Gap, 3),
        (Flat, 6),
        (Up22, 1),
        (Flat, 8),
        (Gap, 6),
        (Flat, 10),
        (Down, 3),
        (Flat, 5),
        (Spikes, 3),
        (Flat, 8),
        (Gap, 8),
        (Flat, 8),
        (Up, 4),
        (Flat, 8),
        (Gap, 5),
        (Flat, 6),
        (Down, 3),
        (Flat, 8),
        (Gap, 3),
        (Flat, 7),
        (Down, 2),
        (Flat, 8),
        (Gap, 6),
        (Flat, 27),
    ]);
    b.walls();
    // Cloud ledges across the wide gaps, level with the neighbouring ground.
    b.plat(64, 65, 7).plat(127, 128, 8).plat(169, 170, 3);
    b.hover(97.0, 4.5, 103.0, 4.5, 0.3);
    b.row(Cheezie, 6, 5, 6, 1)
        .row(Choc, 20, 3, 9, 2)
        .row(Cheezie, 35, 6, 11, 1)
        .item(Cookie, 38, 11)
        .item(Soda, 40, 11)
        .row(Choc, 43, 2, 9, 2)
        .row(Cheezie, 62, 6, 10, 1)
        .row(Cheezie, 68, 5, 10, 2)
        .row(Cheezie, 85, 5, 8, 1)
        .row(Choc, 98, 3, 8, 2)
        .row(Cheezie, 118, 5, 11, 1)
        .item(Soda, 122, 11)
        .item(Cookie, 127, 10)
        .row(Choc, 130, 3, 11, 2)
        .row(Cheezie, 147, 4, 9, 1)
        .row(Choc, 167, 6, 7, 1)
        .row(Cookie, 185, 2, 6, 3);
    b.fill(195, 195, 4, 5, EXIT);
    for x in [8, 30, 50, 74, 110, 121, 154, 182] {
        b.crys(x);
    }
    b.ent(Kind::Gloop, 26.0)
        .ent(Kind::Gloop, 48.0)
        .ent(Kind::Gloop, 72.0)
        .ent(Kind::Gloop, 180.0);
    b.ent(Kind::Hopper, 57.0)
        .ent(Kind::Hopper, 120.0)
        .ent(Kind::Hopper, 162.0)
        .ent(Kind::Gloop, 154.0)
        .ent(Kind::Pod, 108.0);
    b.ent_at(Kind::Drone, 84.5, 11.5)
        .ent_at(Kind::Drone, 141.0, 12.5);
    b.out(METEOR_MESA, (3.0, 4.0), None)
}

/// Nine floors of a Zarg watchtower, 9 tiles apart: climb ladders, stairs and a lift to the roof for
/// the red key, then come back down to the cookie door that guards the exit.
///
/// A flight of stairs lands beyond the end of its run, so each one delivers Ben to a small pocket of
/// floor that holds the next ladder. Floors that no connector reaches have no slab, which leaves
/// tall atriums instead of rooms nobody can enter.
fn zarg_lookout() -> LevelData {
    const W: i32 = 40;
    const H: i32 = 84;
    // Standing row of floor `k`; the slab under it is the row below.
    let y = |k: i32| 3 + 9 * k;
    let mut b = Builder::new(W, H, 3);
    b.fill(0, W - 1, 0, 2, FILL);
    b.fill(0, 0, 0, 80, BLOCK).fill(W - 1, W - 1, 0, 80, BLOCK);
    // Slabs: whole floors for 1, 3, 5, 6 and the roof; pockets for 2, 4 and 7.
    for k in [1, 3, 5, 6, 8] {
        b.fill(1, W - 2, y(k) - 1, y(k) - 1, FILL);
    }
    b.fill(33, 38, y(2) - 1, y(2) - 1, FILL)
        .fill(1, 5, y(4) - 1, y(4) - 1, FILL)
        .fill(35, 38, y(7) - 1, y(7) - 1, FILL);
    // Ground to floor 1: a ladder, with the cookie door sealing the exit closet at the far end.
    b.ladder(18, y(0), y(1) - 1);
    b.fill(36, 36, 3, y(1) - 2, DOOR_R).fill(38, 38, 3, 4, EXIT);
    // Floor 1 to the floor 2 pocket: stairs rising to the right, then a ladder from the pocket.
    b.stairs_right(24, y(1), 9);
    b.ladder(36, y(2), y(3) - 1);
    // Floor 3 to the floor 4 pocket on the left: stairs rising to the left, then a ladder.
    b.stairs_left(6, y(3), 9);
    b.ladder(3, y(4), y(5) - 1);
    // Floor 5 to floor 6: a lift in the middle that waits two seconds at each end.
    b.fill(18, 20, y(6) - 1, y(6) - 1, EMPTY);
    b.plats.push(
        Platform::new(
            18.0,
            f64::from(y(5)) - 0.5,
            18.0,
            f64::from(y(6)) - 0.5,
            0.1,
        )
        .sized(3.0, 0.5)
        .with_dwell(2.0),
    );
    // Floor 6 to the floor 7 pocket: stairs rising to the right, then the last ladder, to the roof.
    b.stairs_right(26, y(6), 9);
    b.ladder(37, y(7), y(8) - 1);
    b.wall_behind(1, W - 2, 3, y(8) - 1);
    // Snacks along the floors, with bigger prizes at the connectors.
    b.row(Cheezie, 6, 6, y(0), 1)
        .row(Cheezie, 8, 6, y(1), 2)
        .row(Cheezie, 34, 4, y(2), 1)
        .row(Cheezie, 16, 6, y(3), 2)
        .row(Cheezie, 12, 6, y(5), 2)
        .row(Cheezie, 22, 3, y(6), 1)
        .row(Cheezie, 36, 3, y(7), 1)
        .row(Cheezie, 8, 6, y(8), 2)
        .item(Soda, 14, y(1))
        .item(Soda, 24, y(5))
        .item(Cookie, 20, y(3))
        .item(Cookie, 30, y(5))
        .item(Cookie, 28, y(8))
        .row(Choc, 3, 3, y(4), 1)
        .row(Choc, 8, 3, y(6), 2)
        .item(KeyRed, 19, y(8));
    for (x, k) in [(14, 0), (20, 1), (22, 3), (12, 5), (15, 6)] {
        b.map.set(x, y(k), CRYS);
    }
    b.ent_at(Kind::Gloop, 14.0, f64::from(y(0)))
        .ent_at(Kind::Hopper, 24.0, f64::from(y(0)))
        .ent_at(Kind::Gloop, 12.0, f64::from(y(1)))
        .ent_at(Kind::Gloop, 20.0, f64::from(y(3)))
        .ent_at(Kind::Hopper, 30.0, f64::from(y(3)))
        .ent_at(Kind::Gloop, 28.0, f64::from(y(5)))
        .ent_at(Kind::Pod, 8.0, f64::from(y(6)))
        .ent_at(Kind::Gloop, 14.0, f64::from(y(6)))
        .ent_at(Kind::Gloop, 14.0, f64::from(y(8)))
        .ent_at(Kind::Hopper, 26.0, f64::from(y(8)));
    // Bats hang from the slab above the fifth floor.
    for x in [10, 14, 28, 33] {
        b.ent_at(Kind::Bat, f64::from(x), f64::from(y(5)) + 7.3);
    }
    b.out(ZARG_LOOKOUT, (3.0, 3.0), None)
}

/// Open sky with soft ground: marshmallows bounce in fenced corrals (pogo onto one for a springboard),
/// and the blue key sits at the top of a stack of cloud ledges. Rises and falls are slopes; gaps are
/// 3 wide.
fn marshmallow_meadows() -> LevelData {
    let mut b = Builder::new(220, 32, 4);
    b.run(&[
        (Flat, 14),
        (Up, 3),
        (Flat, 8),
        (Down, 3),
        (Flat, 10),
        (Gap, 3),
        (Flat, 12),
        (Up22, 2),
        (Flat, 10),
        (Down22, 2),
        (Flat, 10),
        (Gap, 3),
        (Flat, 8),
        (Up, 2),
        (Flat, 24),
        (Gap, 3),
        (Flat, 14),
        (Down, 3),
        (Flat, 6),
        (Gap, 3),
        (Flat, 10),
        (Up, 4),
        (Flat, 14),
        (Gap, 3),
        (Flat, 10),
        (Down, 2),
        (Flat, 30),
    ]);
    b.walls();
    // Corral fences: a marshmallow bounces between two posts Ben can jump over.
    b.fill(29, 29, 4, 5, BLOCK)
        .fill(36, 36, 4, 5, BLOCK)
        .fill(100, 100, 6, 7, BLOCK)
        .fill(108, 108, 6, 7, BLOCK);
    // The blue key, three cloud ledges up: each step is three tiles, within a jump.
    b.plat(58, 60, 8).plat(62, 64, 11).plat(58, 60, 14);
    b.item(KeyBlue, 59, 15);
    b.fill(150, 150, 3, 4, DOOR_B).fill(150, 150, 5, 31, BLOCK);
    b.fill(215, 215, 5, 6, EXIT);
    b.row(Cheezie, 6, 6, 6, 1)
        .row(Cheezie, 18, 6, 9, 1)
        .row(Cheezie, 30, 6, 9, 1)
        .item(Soda, 33, 7)
        .row(Choc, 38, 3, 8, 1)
        .row(Cheezie, 44, 8, 6, 1)
        .item(Cookie, 63, 12)
        .row(Cheezie, 70, 6, 6, 1)
        .row(Choc, 81, 3, 8, 1)
        .row(Cheezie, 96, 4, 9, 1)
        .row(Cheezie, 102, 5, 11, 1)
        .item(Soda, 104, 12)
        .row(Cheezie, 118, 3, 9, 1)
        .row(Cheezie, 125, 6, 8, 1)
        .item(Cookie, 140, 6)
        .row(Choc, 144, 3, 7, 1)
        .row(Cheezie, 160, 5, 9, 1)
        .row(Choc, 175, 3, 11, 1)
        .item(Soda, 182, 10)
        .row(Cookie, 196, 3, 8, 3);
    for x in [8, 24, 48, 74, 96, 126, 142, 164, 185, 205] {
        b.crys(x);
    }
    b.ent(Kind::Marsh, 32.0).ent(Kind::Marsh, 104.0);
    b.ent(Kind::Gloop, 20.0)
        .ent(Kind::Gloop, 45.0)
        .ent(Kind::Gloop, 74.0)
        .ent(Kind::Gloop, 125.0)
        .ent(Kind::Gloop, 195.0)
        .ent(Kind::Hopper, 60.0)
        .ent(Kind::Hopper, 112.0)
        .ent(Kind::Hopper, 165.0)
        .ent(Kind::Pod, 141.0);
    b.ent_at(Kind::Drone, 90.5, 10.5)
        .ent_at(Kind::Drone, 170.5, 12.5);
    b.out(MARSHMALLOW_MEADOWS, (3.0, 4.0), None)
}

/// A candy playhouse whose backstage rooms are hidden behind painted flats: the walls fade away
/// while Ben is inside. The red key is in a room up a stack of ledges, the blue key in one over a
/// bat-haunted stretch, and a beetle waits in a room on the path.
fn bonbon_playhouse() -> LevelData {
    let mut b = Builder::new(170, 30, 4);
    b.run(&[
        (Flat, 16),
        (Up, 2),
        (Flat, 12),
        (Down, 2),
        (Flat, 10),
        (Gap, 3),
        (Flat, 12),
        (Up22, 1),
        (Flat, 10),
        (Down22, 1),
        (Flat, 14),
        (Gap, 3),
        (Flat, 10),
        (Up, 3),
        (Flat, 14),
        (Down, 3),
        (Flat, 10),
        (Gap, 3),
        (Flat, 14),
        (Flat, 25),
    ]);
    b.walls();
    // Red key room: a floor over the path, reached by ledges that start over the first pit.
    b.fill(45, 56, 9, 9, FILL);
    b.plat(42, 43, 5).plat(44, 44, 7);
    b.item(KeyRed, 50, 10)
        .item(Cookie, 54, 10)
        .row(Cheezie, 46, 4, 10, 1);
    b.ent_at(Kind::Hopper, 52.0, 10.0);
    b.room(45, 10, 56, 15);
    // A tutorial room right on the path, with a gloop lying in wait.
    b.row(Cheezie, 20, 6, 7, 1).item(Soda, 27, 7);
    b.ent_at(Kind::Gloop, 24.0, 6.0);
    b.room(18, 6, 29, 11);
    // A room on the path that hides a beetle; it stops at the ledge at the room's end.
    b.row(Choc, 74, 4, 5, 2);
    b.ent_at(Kind::Beetle, 80.0, 4.0);
    b.room(71, 4, 84, 9);
    // Blue key room over the path, reached by four ledges, with bats hanging underneath.
    b.fill(101, 114, 13, 13, FILL);
    b.plat(92, 93, 6).plat(95, 96, 9).plat(98, 99, 12);
    b.item(KeyBlue, 108, 14)
        .item(Cookie, 104, 14)
        .row(Cheezie, 110, 4, 14, 1);
    b.room(101, 14, 114, 19);
    for x in [104.0, 110.0] {
        b.ent_at(Kind::Bat, x, 11.3);
    }
    // Cookie doors seal the way on, with a wall of painted flats above each.
    b.fill(60, 60, 5, 6, DOOR_R).fill(60, 60, 7, 29, BLOCK);
    b.fill(122, 122, 4, 5, DOOR_B).fill(122, 122, 6, 29, BLOCK);
    b.fill(165, 165, 4, 5, EXIT);
    b.row(Cheezie, 6, 6, 6, 1)
        .row(Choc, 34, 3, 7, 1)
        .row(Cheezie, 62, 6, 8, 1)
        .item(Soda, 66, 9)
        .row(Cheezie, 88, 3, 8, 2)
        .row(Cheezie, 118, 4, 8, 1)
        .row(Choc, 134, 4, 7, 2)
        .row(Cookie, 148, 3, 7, 3);
    for x in [10, 36, 64, 90, 118, 138, 158] {
        b.crys(x);
    }
    b.ent(Kind::Gloop, 10.0)
        .ent(Kind::Gloop, 36.0)
        .ent(Kind::Gloop, 66.0)
        .ent(Kind::Gloop, 92.0)
        .ent(Kind::Gloop, 120.0)
        .ent(Kind::Gloop, 140.0)
        .ent(Kind::Pod, 63.0)
        .ent(Kind::Pod, 135.0);
    b.out(BONBON_PLAYHOUSE, (3.0, 4.0), None)
}

/// A harder crater: wide fudge pools crossed on stepping-stone ledges, spikes, three spore pods and a
/// red key at the top of a short tower of ledges.
fn fudge_bog() -> LevelData {
    let mut b = Builder::new(190, 28, 4);
    b.run(&[
        (Flat, 12),
        (Pool, 3),
        (Flat, 8),
        (Up, 2),
        (Flat, 6),
        (Pool, 5),
        (Flat, 10),
        (Spikes, 3),
        (Flat, 8),
        (Down, 2),
        (Pool, 3),
        (Flat, 12),
        (Up22, 2),
        (Flat, 8),
        (Pool, 6),
        (Flat, 10),
        (Spikes, 3),
        (Flat, 9),
        (Pool, 3),
        (Flat, 10),
        (Up, 3),
        (Flat, 14),
        (Down, 3),
        (Pool, 4),
        (Flat, 39),
    ]);
    b.walls();
    b.pogo_cache(18, 20, 7);
    // Stepping stones over the wide pools, level with the ground on either side.
    b.plat(32, 33, 5).plat(88, 89, 5).plat(148, 149, 5);
    // The red key: two ledges, each three tiles above the last.
    b.plat(134, 136, 11).plat(138, 140, 14);
    b.item(KeyRed, 139, 15);
    b.fill(165, 165, 6, 7, DOOR_R).fill(165, 165, 8, 27, BLOCK);
    b.fill(185, 185, 6, 7, EXIT);
    b.row(Cheezie, 4, 6, 6, 1)
        .row(Choc, 12, 3, 8, 1)
        .row(Cheezie, 26, 5, 8, 1)
        .row(Choc, 31, 5, 9, 1)
        .row(Cheezie, 37, 6, 8, 1)
        .row(Cheezie, 46, 3, 9, 1)
        .item(Soda, 52, 8)
        .row(Choc, 59, 3, 7, 1)
        .row(Cheezie, 64, 8, 6, 1)
        .item(Cookie, 80, 9)
        .row(Choc, 86, 6, 9, 1)
        .row(Cheezie, 94, 6, 8, 1)
        .row(Cheezie, 102, 3, 9, 1)
        .item(Soda, 108, 8)
        .row(Cheezie, 117, 8, 8, 1)
        .item(Cookie, 135, 12)
        .item(Cookie, 140, 15)
        .row(Choc, 147, 4, 8, 1)
        .row(Cheezie, 152, 6, 8, 1)
        .row(Cookie, 172, 4, 8, 3);
    for x in [6, 20, 40, 66, 82, 96, 120, 136, 156, 176] {
        b.crys(x);
    }
    b.ent(Kind::Pod, 18.0)
        .ent(Kind::Pod, 67.0)
        .ent(Kind::Pod, 121.0);
    b.ent(Kind::Gloop, 28.0)
        .ent(Kind::Gloop, 96.0)
        .ent(Kind::Gloop, 155.0)
        .ent(Kind::Gloop, 175.0)
        .ent(Kind::Hopper, 40.0)
        .ent(Kind::Hopper, 82.0)
        .ent(Kind::Hopper, 133.0)
        .ent(Kind::Beetle, 109.0);
    b.ent_at(Kind::Drone, 60.5, 9.5)
        .ent_at(Kind::Drone, 125.5, 11.5)
        .ent_at(Kind::Drone, 170.5, 11.5);
    // A rock outcrop over the first stretch of dry land hides a room: climb the vine, then shoot
    // the cracked wall at the end of the pocket.
    b.fill(37, 45, 10, 18, FILL);
    let foot = b.ground_at(36.5) as i32;
    b.vine_nook(36, foot, 12, 1, 5, 4);
    b.item(Cookie, 41, 12)
        .item(Cookie, 43, 12)
        .item(Soda, 40, 12)
        .ent_dir(Kind::Glyph, 38.0, f64::from(foot), 3.0)
        .ent_dir(Kind::Cameo, 44.0, 12.0, 1.0);
    b.out(FUDGE_BOG, (3.0, 4.0), None)
}

/// A tall crystal shaft sealed by three gates. Each gate opens when a fizz bubble hits a crystal
/// switch Ben cannot shoot directly, so he fires up and lets mirrors carry the bubble round the
/// corner: a single mirror, then two in a chain, then a swivel mirror that sends the first bubble
/// the wrong way and the second one right. A bubble flies about 17 tiles, which every route fits.
/// A ledge hugs the underside of each gate, so Ben can only climb on once it opens.
fn mirror_shafts() -> LevelData {
    const W: i32 = 36;
    const H: i32 = 100;
    let mut b = Builder::new(W, H, 3);
    b.fill(0, W - 1, 0, 2, FILL)
        .fill(0, 2, 0, H - 1, FILL)
        .fill(W - 3, W - 1, 0, H - 1, FILL);
    // Ledges, written as (standing row, first column, last column).
    let ledges: [(i32, i32, i32); 30] = [
        (6, 4, 12),
        (9, 10, 18),
        (12, 16, 24),
        (15, 22, 26),
        (18, 16, 24),
        (21, 10, 26),
        (24, 4, 12),
        (27, 8, 16),
        (30, 12, 20),
        (33, 6, 16),
        (36, 12, 22),
        (39, 18, 28),
        (42, 12, 22),
        (45, 6, 16),
        (48, 10, 20),
        (51, 6, 14),
        (54, 6, 14),
        (57, 12, 17),
        (60, 19, 25),
        (63, 24, 30),
        (66, 18, 28),
        (69, 14, 24),
        (72, 12, 22),
        (75, 18, 28),
        (78, 12, 24),
        (81, 24, 32),
        (84, 18, 26),
        (87, 18, 26),
        (90, 14, 24),
        (93, 16, 24),
    ];
    for (stand, x0, x1) in ledges {
        b.plat(x0, x1, stand - 1);
    }
    // Three gates, each on its own switch channel and closed to start with.
    b.fill(3, 32, 30, 31, GATE)
        .fill(3, 32, 66, 67, GATE_2)
        .fill(3, 32, 87, 88, GATE_3);
    for ch in [GATE_CHANNEL, 2, 3] {
        b.map.set_switch(ch, true);
    }
    // Puzzle 1: fire up from the wide ledge at row 21; one mirror turns the bubble right.
    b.ent_dir(Kind::Mirror, 18.0, 26.0, 1.0)
        .ent_dir(Kind::CrystalSwitch, 28.0, 26.0, 1.0);
    // Puzzle 2: two mirrors in a chain, right and then up.
    b.ent_dir(Kind::Mirror, 10.0, 58.0, 1.0)
        .ent_dir(Kind::Mirror, 17.0, 58.0, 1.0)
        .ent_dir(Kind::CrystalSwitch, 17.0, 62.0, 2.0);
    // Puzzle 3: the swivel mirror sends the first bubble right into the wall and swings round to
    // send the second left, to the switch.
    b.ent_dir(Kind::Swivel, 18.0, 82.0, 1.0)
        .ent_dir(Kind::CrystalSwitch, 8.0, 82.0, 3.0);
    // Shield the switches. Each bubble reaches its switch along a short corridor of rock that is
    // open only where the mirror sends the bubble in, so nobody can reach a switch by jumping and
    // firing sideways from a ledge: puzzle one's and three's switches sit at the end of a
    // corridor, puzzle two's in a niche the bubble enters from below.
    b.fill(19, 28, 25, 25, FILL).fill(19, 28, 27, 27, FILL);
    b.fill(16, 16, 61, 63, FILL).fill(18, 18, 61, 63, FILL);
    b.fill(4, 17, 81, 81, FILL).fill(4, 17, 83, 83, FILL);
    b.fill(20, 20, 93, 94, EXIT);
    // A deliberate shortcut past gate 2: from the ledge under it, a cracked wall in the right-hand
    // rock opens onto a vine that climbs inside the wall and comes out on top of the gate. Two
    // cookies pay for finding it.
    b.plat(31, 32, 62);
    b.fill(33, 34, 63, 64, EMPTY)
        .fill(34, 34, 63, 69, EMPTY)
        .fill(33, 33, 68, 69, EMPTY)
        .fill(33, 33, 63, 64, CRACKED)
        .fill(34, 34, 63, 69, VINE)
        .item(Cookie, 34, 65)
        .item(Cookie, 34, 68)
        .ent_dir(Kind::Glyph, 26.0, 63.0, 2.0);
    // Soda by each firing spot, so a few wasted bubbles never strand Ben.
    b.item(Soda, 13, 21)
        .item(Soda, 8, 54)
        .item(Soda, 21, 78)
        .item(Soda, 12, 6);
    for (stand, x0, x1) in ledges {
        // The ledges under gates 1 and 2 have the gate's own rows at head height: no snacks there.
        if stand % 6 == 0 && stand != 30 && stand != 66 {
            b.row(Cheezie, x0 + 1, (x1 - x0 - 1).min(5), stand, 1);
        }
    }
    b.row(Choc, 12, 3, 22, 2)
        .item(Cookie, 22, 40)
        .item(Cookie, 24, 61)
        .item(Cookie, 22, 76)
        .row(Choc, 18, 3, 91, 2);
    for (x, y) in [(10, 8), (24, 17), (20, 41), (12, 50), (26, 74), (20, 91)] {
        b.map.set(x, y, CRYS);
    }
    b.ent_at(Kind::Phantom, 26.0, 15.0)
        .ent_at(Kind::Phantom, 22.0, 39.0)
        .ent_at(Kind::Phantom, 16.0, 72.0)
        .ent_at(Kind::Pod, 14.0, 48.0)
        .ent_at(Kind::Pod, 24.0, 75.0)
        .ent_at(Kind::Drone, 20.5, 36.5)
        .ent_at(Kind::Drone, 14.5, 68.5);
    b.out(MIRROR_SHAFTS, (6.0, 3.0), None)
}

/// A long theatre of stacked hidden rooms. Red and blue keys sit in rooms up on balconies, a beetle
/// waits in a room on the path, and an alcove reached by ledges over the last pit holds a mural of
/// the crystal forest: the secret clue.
fn sugar_glass_gallery() -> LevelData {
    let mut b = Builder::new(160, 30, 4);
    b.run(&[
        (Flat, 14),
        (Up, 2),
        (Flat, 10),
        (Down, 2),
        (Flat, 8),
        (Gap, 3),
        (Flat, 12),
        (Up22, 1),
        (Flat, 10),
        (Down22, 1),
        (Flat, 12),
        (Gap, 3),
        (Flat, 10),
        (Up, 3),
        (Flat, 14),
        (Down, 3),
        (Flat, 10),
        (Gap, 3),
        (Flat, 14),
        (Flat, 23),
    ]);
    b.walls();
    // An ambush room on the path: two gloops lying in wait.
    b.row(Cheezie, 18, 6, 7, 1);
    b.ent_at(Kind::Gloop, 20.0, 6.0)
        .ent_at(Kind::Gloop, 23.0, 6.0);
    b.room(16, 6, 25, 11);
    // Red key balcony, reached by ledges over the first pit.
    b.fill(41, 50, 9, 9, FILL);
    b.plat(36, 37, 5).plat(38, 38, 8);
    b.item(KeyRed, 46, 10)
        .item(Cookie, 48, 10)
        .row(Cheezie, 42, 3, 10, 1);
    b.ent_at(Kind::Bat, 45.0, 7.3);
    b.room(41, 10, 50, 15);
    // A beetle room on the path, ending at a ledge so the charge stops short of the pit.
    b.row(Choc, 67, 4, 5, 2);
    b.ent_at(Kind::Beetle, 74.0, 4.0);
    b.room(65, 4, 76, 9);
    // Blue key balcony, up four ledges, with two bats hanging beneath it.
    b.fill(95, 106, 13, 13, FILL);
    b.plat(86, 87, 6).plat(89, 90, 9).plat(92, 93, 12);
    b.item(KeyBlue, 100, 14)
        .item(Cookie, 104, 14)
        .row(Cheezie, 96, 3, 14, 1);
    b.room(95, 14, 106, 19);
    for x in [98.0, 103.0] {
        b.ent_at(Kind::Bat, x, 11.3);
    }
    // The secret: an alcove over the last pit, breadcrumbed with snacks, holding a mural.
    b.fill(126, 136, 9, 9, FILL);
    b.plat(120, 121, 5).plat(122, 123, 8);
    b.row(Cheezie, 119, 3, 7, 1).row(Cheezie, 124, 2, 10, 1);
    b.mural(130, 11);
    b.row(Cookie, 133, 3, 10, 1);
    b.secret_room(126, 10, 136, 15);
    // Doors, and the exit.
    b.fill(60, 60, 5, 6, DOOR_R).fill(60, 60, 7, 29, BLOCK);
    b.fill(112, 112, 4, 5, DOOR_B).fill(112, 112, 6, 29, BLOCK);
    b.fill(155, 155, 4, 5, EXIT);
    b.row(Cheezie, 5, 6, 6, 1)
        .row(Choc, 29, 4, 7, 1)
        .row(Cheezie, 54, 5, 8, 1)
        .item(Soda, 58, 9)
        .row(Cheezie, 80, 6, 8, 1)
        .item(Soda, 114, 7)
        .row(Cheezie, 138, 6, 8, 1)
        .row(Cookie, 146, 3, 7, 3);
    for x in [8, 30, 56, 82, 114, 140] {
        b.crys(x);
    }
    b.ent(Kind::Gloop, 8.0)
        .ent(Kind::Gloop, 31.0)
        .ent(Kind::Gloop, 56.0)
        .ent(Kind::Gloop, 70.0)
        .ent(Kind::Gloop, 84.0)
        .ent(Kind::Gloop, 128.0)
        .ent(Kind::Gloop, 144.0)
        .ent(Kind::Phantom, 148.0)
        .ent(Kind::Pod, 55.0)
        .ent(Kind::Pod, 115.0);
    b.out(SUGAR_GLASS_GALLERY, (3.0, 4.0), None)
}

/// The hardest open-sky level: spike runs, a hover platform over an eight-tile gap and a chain of
/// two over a ten-tile one, with drones overhead. The red key is at the top of a pair of cloud
/// ledges, and the way on is behind a cookie door.
fn frosting_flats() -> LevelData {
    let mut b = Builder::new(190, 36, 4);
    b.run(&[
        (Flat, 12),
        (Gap, 3),
        (Flat, 8),
        (Up, 3),
        (Flat, 6),
        (Gap, 8),
        (Flat, 8),
        (Spikes, 3),
        (Flat, 6),
        (Down, 3),
        (Flat, 8),
        (Gap, 3),
        (Flat, 10),
        (Up22, 2),
        (Flat, 8),
        (Gap, 10),
        (Flat, 10),
        (Spikes, 3),
        (Flat, 5),
        (Gap, 3),
        (Flat, 12),
        (Down22, 1),
        (Flat, 8),
        (Gap, 6),
        (Flat, 38),
    ]);
    b.walls();
    // A cloud ledge across the six-tile gap, level with the ground.
    b.plat(148, 149, 4);
    // One hover over the eight-tile gap; two in a chain over the ten-tile gap, the second one
    // half a lap ahead so Ben can hop from one to the other.
    b.hover(32.0, 6.5, 38.0, 6.5, 0.3);
    b.hover(93.0, 5.5, 97.0, 5.5, 0.3);
    b.hover(97.0, 5.5, 101.0, 5.5, 0.3);
    // The red key, two cloud ledges up.
    b.plat(126, 128, 8).plat(130, 132, 11);
    b.item(KeyRed, 131, 12);
    b.fill(160, 160, 5, 6, DOOR_R).fill(160, 160, 7, 35, BLOCK);
    b.fill(182, 182, 5, 6, EXIT);
    b.row(Cheezie, 4, 6, 6, 1)
        .row(Choc, 12, 3, 8, 1)
        .row(Cheezie, 16, 6, 6, 1)
        .row(Cheezie, 26, 5, 9, 1)
        .item(Soda, 30, 10)
        .row(Choc, 34, 4, 10, 2)
        .row(Cheezie, 41, 6, 9, 1)
        .row(Cheezie, 48, 3, 10, 1)
        .row(Cheezie, 61, 6, 6, 1)
        .row(Choc, 68, 3, 7, 1)
        .item(Cookie, 76, 7)
        .row(Cheezie, 86, 6, 8, 1)
        .row(Choc, 94, 8, 9, 1)
        .row(Cheezie, 104, 8, 8, 1)
        .row(Cheezie, 113, 3, 9, 1)
        .item(Soda, 118, 8)
        .item(Cookie, 127, 9)
        .item(Cookie, 131, 13)
        .row(Cheezie, 138, 6, 7, 1)
        .row(Choc, 146, 6, 6, 1)
        .row(Cheezie, 154, 5, 7, 1)
        .row(Cookie, 168, 4, 8, 3);
    for x in [8, 28, 44, 62, 76, 88, 108, 130, 142, 164, 178] {
        b.crys(x);
    }
    b.ent(Kind::Gloop, 25.0)
        .ent(Kind::Gloop, 88.0)
        .ent(Kind::Gloop, 118.0)
        .ent(Kind::Gloop, 172.0)
        .ent(Kind::Hopper, 18.0)
        .ent(Kind::Hopper, 64.0)
        .ent(Kind::Hopper, 106.0)
        .ent(Kind::Hopper, 130.0)
        .ent(Kind::Hopper, 165.0)
        .ent(Kind::Pod, 74.0)
        .ent(Kind::Pod, 141.0);
    b.ent_at(Kind::Drone, 20.5, 10.5)
        .ent_at(Kind::Drone, 60.5, 10.5)
        .ent_at(Kind::Drone, 108.5, 12.5)
        .ent_at(Kind::Drone, 170.5, 11.5);
    b.out(FROSTING_FLATS, (3.0, 4.0), None)
}

/// Twelve floors of the tallest Zarg tower, with all three key colours. Ladders alternate sides up
/// the tower and three lifts carry Ben between floors. A red door splits the fifth floor and a blue
/// door the eighth, so he fetches those keys on the way up; the green key is on the roof and its
/// door guards the exit at the bottom, so the climb ends with the whole tower to come down.
fn frosting_spire() -> LevelData {
    const W: i32 = 44;
    const H: i32 = 112;
    let y = |k: i32| 3 + 9 * k;
    // Even connectors have their ladder on the left, odd ones on the right.
    let ladder_x = |k: i32| if k % 2 == 0 { 6 } else { 38 };
    let mut b = Builder::new(W, H, 3);
    b.fill(0, W - 1, 0, 2, FILL)
        .fill(0, 0, 0, 108, BLOCK)
        .fill(W - 1, W - 1, 0, 108, BLOCK);
    for k in 1..=11 {
        b.fill(1, W - 2, y(k) - 1, y(k) - 1, FILL);
    }
    for k in 0..=10 {
        if matches!(k, 2 | 6 | 9) {
            b.fill(20, 22, y(k + 1) - 1, y(k + 1) - 1, EMPTY);
            b.plats.push(
                Platform::new(
                    20.0,
                    f64::from(y(k)) - 0.5,
                    20.0,
                    f64::from(y(k + 1)) - 0.5,
                    0.14,
                )
                .sized(3.0, 0.5)
                .with_dwell(1.5),
            );
        } else {
            b.ladder(ladder_x(k), y(k), y(k + 1) - 1);
        }
    }
    b.wall_behind(1, W - 2, 3, y(11) - 1);
    // Doors: red splits floor 5, blue splits floor 8, green seals the ground-floor exit closet.
    b.fill(21, 21, y(5), y(5) + 7, DOOR_R)
        .fill(21, 21, y(8), y(8) + 7, DOOR_B)
        .fill(36, 36, 3, 10, DOOR_G)
        .fill(42, 42, 3, 4, EXIT);
    b.item(KeyRed, 14, y(3))
        .item(KeyBlue, 14, y(7))
        .item(KeyGreen, 30, y(11));
    for k in 0..=11 {
        b.row(Cheezie, if k % 2 == 0 { 10 } else { 26 }, 5, y(k), 1);
    }
    b.item(Soda, 12, y(1))
        .item(Soda, 30, y(4))
        .item(Soda, 12, y(9))
        .item(Cookie, 30, y(2))
        .item(Cookie, 14, y(6))
        .item(Cookie, 28, y(10))
        .row(Choc, 24, 3, y(5), 2)
        .row(Choc, 24, 3, y(8), 2);
    for (x, k) in [
        (14, 0),
        (28, 1),
        (10, 3),
        (30, 4),
        (14, 5),
        (30, 7),
        (10, 9),
        (26, 11),
    ] {
        b.map.set(x, y(k), CRYS);
    }
    for (x, k, kind) in [
        (14, 0, Kind::Gloop),
        (24, 0, Kind::Hopper),
        (20, 1, Kind::Gloop),
        (30, 2, Kind::Hopper),
        (24, 3, Kind::Gloop),
        (30, 3, Kind::Beetle),
        (14, 4, Kind::Gloop),
        (14, 6, Kind::Hopper),
        (28, 6, Kind::Gloop),
        (12, 7, Kind::Beetle),
        (32, 8, Kind::Gloop),
        (20, 9, Kind::Hopper),
        (14, 10, Kind::Gloop),
        (36, 11, Kind::Hopper),
    ] {
        b.ent_at(kind, f64::from(x), f64::from(y(k)));
    }
    // Sentries hover on the floors that follow each door and on the way to the roof.
    for k in [5, 8, 10] {
        b.ent_at(Kind::Sentry, 30.0, f64::from(y(k)) + 2.5);
    }
    b.ent_at(Kind::Phantom, 30.0, f64::from(y(6)))
        .ent_at(Kind::Phantom, 12.0, f64::from(y(9)));
    // Bats hang under the slab above floors 1, 4 and 7.
    for (x, k) in [(14, 1), (30, 4), (14, 7), (30, 9)] {
        b.ent_at(Kind::Bat, f64::from(x), f64::from(y(k)) + 7.3);
    }
    b.out(FROSTING_SPIRE, (3.0, 3.0), None)
}

/// The cocoa foundry: conveyor belts that help and hinder, crushing presses to time, pools of molten
/// metal crossed on a hover platform and stepping stones, and the blue key up a short tower of
/// ledges. Presses are harmless while raised, so every one can be walked under in its window.
fn cocoa_foundry() -> LevelData {
    let mut b = Builder::new(214, 30, 4);
    b.liquid = FURNACE;
    b.run(&[
        (Flat, 12),
        (BeltR, 10),
        (Flat, 5),
        (BeltL, 10),
        (Flat, 6),
        (Pool, 3),
        (Flat, 8),
        (Up, 2),
        (Flat, 6),
        (BeltR, 14),
        (Flat, 4),
        (Pool, 7),
        (Flat, 8),
        (BeltL, 10),
        (Flat, 7),
        (Spikes, 3),
        (Flat, 8),
        (Down, 2),
        (Flat, 8),
        (Pool, 3),
        (Flat, 10),
        (BeltR, 14),
        (Flat, 8),
        (Pool, 6),
        (Flat, 40),
    ]);
    b.walls();
    // Over the seven-tile furnace: one hover. Over the six-tile one: stepping stones.
    b.hover(80.0, 5.5, 85.0, 5.5, 0.28);
    b.plat(170, 171, 3);
    // The blue key, two ledges up, each three tiles above the last.
    b.plat(88, 90, 8).plat(92, 94, 11);
    b.item(KeyBlue, 93, 12);
    b.fill(180, 180, 4, 5, DOOR_B).fill(180, 180, 6, 29, BLOCK);
    b.fill(208, 208, 4, 5, EXIT);
    // A press every so often, each on a floor it can crush; the floor height is read from the map.
    for x in [
        24, 40, 50, 58, 68, 78, 100, 108, 118, 128, 140, 152, 164, 176, 186, 196,
    ] {
        let floor = (0..b.h)
            .rev()
            .find(|&y| b.map.solid(x, y, false, 0.0) || b.map.get(x, y) == PLAT)
            .map_or(4, |y| y + 1);
        b.ent_at(Kind::Press, f64::from(x), f64::from(floor) + 3.0);
    }
    b.row(Cheezie, 4, 6, 6, 1)
        .row(Cheezie, 12, 10, 6, 1)
        .row(Choc, 28, 8, 6, 1)
        .row(Cheezie, 38, 5, 6, 1)
        .row(Choc, 43, 3, 7, 1)
        .item(Soda, 49, 6)
        .row(Cheezie, 62, 14, 8, 1)
        .row(Choc, 80, 7, 9, 1)
        .item(Cookie, 90, 9)
        .item(Cookie, 94, 12)
        .row(Cheezie, 96, 10, 8, 1)
        .row(Cheezie, 105, 6, 8, 1)
        .item(Soda, 118, 8)
        .row(Cheezie, 125, 6, 6, 1)
        .row(Choc, 133, 3, 7, 1)
        .row(Cheezie, 146, 14, 6, 1)
        .row(Choc, 168, 6, 6, 1)
        .item(Soda, 175, 6)
        .row(Cheezie, 182, 8, 6, 2)
        .row(Cookie, 198, 3, 6, 3);
    for x in [6, 30, 52, 70, 90, 108, 130, 150, 172, 190, 204] {
        b.crys(x);
    }
    b.ent(Kind::Gloop, 24.0)
        .ent(Kind::Gloop, 48.0)
        .ent(Kind::Gloop, 120.0)
        .ent(Kind::Gloop, 188.0)
        .ent(Kind::Beetle, 59.0)
        .ent(Kind::Beetle, 130.0)
        .ent(Kind::Beetle, 163.0)
        .ent(Kind::Phantom, 98.0)
        .ent(Kind::Phantom, 142.0);
    b.ent_at(Kind::Sentry, 108.0, 10.5)
        .ent_at(Kind::Sentry, 192.0, 9.5)
        .ent_at(Kind::Drone, 60.5, 12.5)
        .ent_at(Kind::Drone, 140.5, 11.5);
    b.out(COCOA_FOUNDRY, (3.0, 4.0), None)
}

/// The secret bonus level: a short, snack-dense open-sky course with two caches of twenty-one cookies
/// (each worth an extra life) and no keys. The platforming is tough, the reward generous.
fn gumdrop_isle() -> LevelData {
    let mut b = Builder::new(124, 40, 4);
    b.run(&[
        (Flat, 10),
        (Gap, 3),
        (Flat, 6),
        (Gap, 8),
        (Flat, 6),
        (Up, 3),
        (Flat, 5),
        (Gap, 3),
        (Flat, 6),
        (Gap, 10),
        (Flat, 8),
        (Down, 3),
        (Flat, 6),
        (Gap, 3),
        (Flat, 8),
        (Gap, 6),
        (Flat, 30),
    ]);
    b.walls();
    b.hover(19.0, 3.5, 24.0, 3.5, 0.3);
    b.hover(50.0, 6.5, 54.0, 6.5, 0.3);
    b.hover(54.0, 6.5, 58.0, 6.5, 0.3);
    b.plat(90, 91, 3);
    // First cache: three rows of seven cookies after the hover chain.
    b.row(Cookie, 60, 7, 8, 1)
        .row(Cookie, 60, 7, 9, 1)
        .row(Cookie, 60, 7, 10, 1);
    // Second cache: three more rows on the long final flat.
    b.row(Cookie, 100, 7, 5, 1)
        .row(Cookie, 100, 7, 6, 1)
        .row(Cookie, 100, 7, 7, 1);
    b.row(Cheezie, 2, 7, 6, 1)
        .row(Choc, 10, 3, 8, 1)
        .row(Cheezie, 14, 5, 6, 1)
        .row(Choc, 19, 8, 8, 1)
        .item(Soda, 30, 6)
        .row(Cheezie, 28, 5, 8, 1)
        .row(Choc, 41, 3, 10, 1)
        .row(Cheezie, 45, 5, 9, 1)
        .row(Choc, 50, 10, 10, 1)
        .row(Cheezie, 72, 5, 6, 1)
        .row(Choc, 77, 3, 7, 1)
        .row(Cheezie, 81, 6, 6, 1)
        .row(Choc, 88, 6, 6, 1)
        .row(Cheezie, 94, 5, 6, 1)
        .item(Soda, 98, 6)
        .row(Choc, 110, 6, 5, 1);
    b.fill(120, 120, 4, 5, EXIT);
    for x in [6, 16, 30, 46, 64, 82, 100, 116] {
        b.crys(x);
    }
    b.ent(Kind::Gloop, 8.0)
        .ent(Kind::Gloop, 74.0)
        .ent(Kind::Gloop, 110.0)
        .ent(Kind::Hopper, 15.0)
        .ent(Kind::Hopper, 38.0)
        .ent(Kind::Hopper, 62.0)
        .ent(Kind::Hopper, 82.0)
        .ent(Kind::Pod, 46.0);
    b.ent_at(Kind::Drone, 28.5, 9.5)
        .ent_at(Kind::Drone, 48.5, 12.5)
        .ent_at(Kind::Drone, 100.5, 10.5);
    b.out(GUMDROP_ISLE, (3.0, 4.0), None)
}

fn caves() -> LevelData {
    let mut b = Builder::new(176, 26, 5);
    b.run(&[
        (Flat, 12),
        (Down22, 1),
        (Flat, 8),
        (Pool, 8),
        (Flat, 10),
        (Up, 2),
        (Flat, 10),
        (Down, 2),
        (Flat, 6),
        (Pool, 14),
        (Flat, 10),
        (Up22, 2),
        (Flat, 8),
        (Spikes, 3),
        (Flat, 12),
        (Down, 3),
        (Flat, 6),
        (Pool, 10),
        (Flat, 8),
        (Up, 3),
        (Flat, 12),
        (Down22, 1),
        (Flat, 20),
    ]);
    b.fill(1, 40, 13, 25, FILL)
        .fill(41, 80, 15, 25, FILL)
        .fill(81, 120, 13, 25, FILL)
        .fill(121, 174, 14, 25, FILL);
    b.fill(30, 33, 11, 12, FILL)
        .fill(95, 97, 11, 12, FILL)
        .fill(160, 161, 12, 13, FILL);
    b.walls();
    b.hover(22.0, 4.0, 27.0, 4.0, 0.3)
        .hover(120.0, 3.5, 123.0, 6.0, 0.4)
        .hover(125.0, 6.0, 128.0, 3.5, 0.4);
    // A beetle rides the first hover platform back and forth over the pool.
    b.rider(Kind::Beetle, 23.0, 4.6);
    b.ent_at(Kind::Switch, 57.0, 4.0);
    b.fill(60, 73, 3, 3, BRIDGE);
    b.row(Cheezie, 4, 5, 6, 1)
        .row(Cheezie, 22, 6, 7, 1)
        .item(Soda, 34, 5)
        .row(Choc, 43, 4, 8, 2)
        .row(Cheezie, 61, 7, 6, 2);
    b.plat(100, 103, 9)
        .item(KeyBlue, 101, 10)
        .row(Choc, 88, 3, 8, 2)
        .item(Cookie, 112, 7)
        .row(Cheezie, 120, 5, 9, 2)
        .item(Soda, 140, 7);
    b.fill(150, 150, 6, 13, DOOR_B);
    b.fill(170, 170, 5, 6, EXIT).row(Cookie, 158, 2, 7, 3);
    for x in [8, 18, 36, 47, 76, 92, 108, 133, 146, 165] {
        b.crys(x);
    }
    for (x, y) in [(50, 14), (86, 12), (118, 12), (138, 13)] {
        b.map.set(x, y, CRYS);
    }
    // Bats hang from the ceiling; their y is a search start (the prototype lowers it by 0.7).
    for (x, y) in [
        (35.0, 12.0),
        (66.0, 14.0),
        (92.0, 12.0),
        (105.0, 12.0),
        (158.0, 13.0),
    ] {
        b.ent_at(Kind::Bat, x, y - 0.7);
    }
    b.ent(Kind::Beetle, 46.0)
        .ent(Kind::Beetle, 145.0)
        .ent(Kind::Hopper, 78.0)
        .ent(Kind::Gloop, 16.0);
    b.ent(Kind::Gloop, 102.0)
        .ent(Kind::Pod, 108.0)
        .ent_at(Kind::Drone, 124.5, 8.5);
    b.pogo_cache(5, 7, 7);
    // A hidden room in the ceiling: a vine climbs to a shelf, and a cracked wall hides the room.
    let vx = 89;
    let foot = b.ground_at(f64::from(vx)) as i32;
    b.vine_nook(vx, foot, 13, -1, 8, 4);
    // The room's far wall: the ceiling steps down here, so close the gap above the old step.
    b.fill(77, 77, 13, 14, FILL);
    b.row(Cookie, 79, 2, 14, 3)
        .row(Choc, 80, 2, 14, 1)
        .item(Soda, 84, 14)
        .ent_dir(Kind::Glyph, 91.0, f64::from(foot), 2.0)
        .ent_dir(Kind::Cameo, 82.0, 13.0, 0.0);
    // Cave paintings and a hanging candy lantern, for anyone who stops to look.
    let g = |x: f64| b.ground_at(x);
    let (g0, g1, g2) = (g(20.5), g(104.5), g(131.5));
    b.ent_dir(Kind::Glyph, 20.0, g0, 0.0)
        .ent_dir(Kind::Glyph, 104.0, g1, 1.0)
        .ent_dir(Kind::Target, 131.0, g2 + 0.9, 5.0);
    b.out(CAVES, (3.0, 5.0), None)
}

/// A cave built around climbing, carved out of solid rock. A rope of vines carries Ben over a
/// fudge pool hand over hand, a three-tile chimney is climbed by kicking between its walls (with
/// a vine to finish on), hanging vines swing him across a chasm, and tall shelves are pulled up
/// onto. A cracked wall in the chimney hides a closet of cookies.
fn whisper_hollow() -> LevelData {
    let mut b = Builder::new(112, 40, 3);
    b.fill(0, 111, 0, 39, FILL);
    // The starting hall, with a fudge pool in the middle that only the rope crosses.
    b.fill(1, 29, 3, 11, EMPTY)
        .fill(8, 22, 1, 2, CHOC)
        .fill(7, 7, 3, 8, VINE)
        .fill(23, 23, 3, 8, VINE)
        .fill(7, 23, 8, 8, VINE);
    // The chimney: a door at the bottom, then three tiles between two walls all the way up.
    b.fill(30, 30, 3, 5, EMPTY)
        .fill(31, 33, 3, 25, EMPTY)
        .fill(31, 31, 14, 25, VINE);
    // A tunnel at the top leads on to the chasm.
    b.fill(34, 48, 22, 25, EMPTY);
    // The closet in the chimney's right-hand wall.
    b.fill(35, 38, 10, 13, EMPTY).fill(34, 34, 10, 11, CRACKED);
    // The chasm: fudge below, a ledge each side, and vines hanging from the ceiling.
    b.fill(49, 86, 3, 34, EMPTY)
        .fill(49, 86, 1, 2, CHOC)
        .fill(49, 52, 3, 21, FILL)
        .fill(80, 86, 3, 21, FILL);
    for x in [56, 60, 64, 68, 72, 76] {
        b.fill(x, x, 20, 34, VINE);
    }
    // The last climb: shelves four tiles apart, to be pulled up onto.
    b.fill(87, 110, 22, 37, EMPTY)
        .fill(90, 92, 22, 25, FILL)
        .fill(95, 97, 22, 29, FILL)
        .fill(100, 102, 22, 33, FILL)
        .fill(103, 110, 22, 33, FILL)
        .fill(109, 109, 34, 35, EXIT);
    // Snacks, ammo and a few beasts.
    b.row(Cheezie, 3, 5, 3, 1)
        .item(Soda, 27, 3)
        .row(Cheezie, 10, 5, 7, 3)
        .row(Cheezie, 32, 1, 8, 1)
        .item(Cookie, 36, 10)
        .item(Cookie, 37, 12)
        .item(Soda, 35, 12)
        .row(Cheezie, 38, 4, 23, 3)
        .item(Soda, 50, 22)
        .row(Cheezie, 58, 4, 27, 4)
        .item(Cookie, 66, 28)
        .row(Choc, 82, 3, 22, 1)
        .item(Soda, 91, 26)
        .item(Cookie, 96, 30)
        .row(Cheezie, 101, 1, 34, 1);
    for (x, y) in [(5, 3), (27, 3), (36, 22), (84, 22), (98, 22)] {
        b.map.set(x, y, CRYS);
    }
    b.ent_at(Kind::Gloop, 26.0, 3.0)
        .ent_at(Kind::Beetle, 42.0, 22.0)
        .ent_at(Kind::Bat, 62.0, 33.0)
        .ent_at(Kind::Bat, 74.0, 33.0)
        .ent_dir(Kind::Glyph, 12.0, 3.0, 0.0)
        .ent_dir(Kind::Glyph, 28.0, 3.0, 2.0)
        .ent_dir(Kind::Glyph, 52.0, 22.0, 3.0);
    b.out(WHISPER_HOLLOW, (3.0, 3.0), None)
}

/// Ben's own saucer: a short walk up to the hatch and a stroll round the inside, with no enemies,
/// no snacks and nothing to clear. Paintings on the walls say what the crew has been up to.
fn saucer() -> LevelData {
    let mut b = Builder::new(72, 22, 3);
    b.fill(0, 71, 0, 2, FILL).walls();
    // The hull: a wall with a hatch at the bottom, a rounded roof, and the far wall.
    b.fill(15, 16, 6, 14, FILL)
        .fill(15, 68, 15, 16, FILL)
        .fill(17, 22, 14, 14, FILL)
        .fill(60, 66, 14, 14, FILL)
        .fill(67, 68, 3, 14, FILL);
    b.wall_behind(17, 66, 3, 13);
    // A ladder up to the bridge loft, which has its own floor.
    b.fill(40, 62, 8, 8, PLAT).ladder(41, 3, 8);
    // Ben's way back out to the map, beside where he lands.
    b.fill(1, 1, 3, 4, EXIT);
    // Crystals light the outside and the hull; the rest is paintings.
    for x in [6, 12, 20, 36, 56] {
        b.map.set(x, 3, CRYS);
    }
    b.ent_dir(Kind::Glyph, 14.0, 3.0, 4.0)
        .ent_dir(Kind::Glyph, 21.0, 3.0, 5.0)
        .ent_dir(Kind::Glyph, 28.0, 3.0, 6.0)
        .ent_dir(Kind::Glyph, 34.0, 3.0, 8.0)
        .ent_dir(Kind::Glyph, 47.0, 9.0, 7.0)
        .ent_dir(Kind::Glyph, 56.0, 9.0, 9.0);
    b.out(SAUCER, (10.0, 3.0), None)
}

fn citadel() -> LevelData {
    let mut b = Builder::new(158, 26, 4);
    b.run(&[
        (Flat, 14),
        (Up22, 2),
        (Flat, 8),
        (Down, 2),
        (Spikes, 3),
        (Flat, 10),
        (Up, 4),
        (Flat, 10),
        (Down, 4),
        (Flat, 12),
        (Pool, 6),
        (Flat, 12),
        (Up22, 2),
        (Flat, 14),
        (Down, 2),
        (Flat, 8),
        (Flat, 40),
    ]);
    b.fill(1, 116, 15, 25, FILL).fill(117, 156, 18, 25, FILL);
    b.fill(117, 117, 8, 17, BLOCK);
    b.fill(148, 156, 4, 8, BLOCK);
    b.walls();
    b.plat(124, 127, 8).plat(137, 140, 8);
    b.row(Cookie, 18, 3, 8, 3)
        .row(Choc, 32, 5, 6, 2)
        .row(Cheezie, 45, 8, 10, 1)
        .item(Soda, 60, 5)
        .row(Cheezie, 71, 6, 7, 1)
        .row(Choc, 80, 4, 6, 2);
    b.plat(96, 99, 10)
        .row(Cookie, 96, 2, 11, 2)
        .item(Soda, 104, 7)
        .row(Cheezie, 109, 6, 6, 1);
    b.fill(10, 10, 4, 4, CRYS);
    b.fill(64, 64, 4, 4, CRYS);
    b.fill(110, 110, 4, 4, CRYS);
    b.ent_at(Kind::Roller, 50.0, 8.0).ent(Kind::Marsh, 36.0);
    b.ent_at(Kind::Sentry, 66.0, 8.0)
        .ent_at(Kind::Sentry, 101.0, 10.0);
    b.ent(Kind::Phantom, 84.0)
        .ent(Kind::Phantom, 112.0)
        .ent(Kind::Beetle, 95.0);
    b.ent_at(Kind::Boss, 138.0, 4.0)
        .ent_at(Kind::Terminal, 120.0, 4.0)
        .ent_at(Kind::Cage, 152.0, 9.0);
    b.out(
        CITADEL,
        (3.0, 4.0),
        Some(Arena {
            x0: 118.0,
            x1: 147.0,
            floor: 4.0,
        }),
    )
}

/// Builds a level by id; an unknown id falls back to the first level.
pub fn build_level(id: u8) -> LevelData {
    let def = LEVELS.get(usize::from(id)).unwrap_or(&LEVELS[0]);
    (def.build)()
}

// ---------- Overworld ----------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PtKind {
    Saucer,
    Level,
    Tele,
    /// A signpost in a town; `level` holds the text number (see `SIGNS` in the shell).
    Sign,
}

#[derive(Clone, Copy, Debug)]
pub struct MapPoint {
    pub kind: PtKind,
    /// Level id for `Level` points.
    pub level: u8,
    pub x: f64,
    pub y: f64,
    /// Index of the paired teleporter.
    pub to: usize,
    /// Levels (as `level_bit`s) that must all be cleared before this point works; 0 means none.
    /// `SECRET_FOUND` can be part of it too.
    pub req: u16,
    pub big: bool,
    /// Not drawn, and not reachable, until `req` is met.
    pub hidden: bool,
}

/// A rectangle of the overworld (inclusive tile coordinates) that belongs to an area.
#[derive(Clone, Copy, Debug)]
pub struct AreaRect {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
    pub area: u8,
}

/// How an area looks on the overworld: its own ground, scatter and river.
#[derive(Clone, Copy, Debug)]
pub struct AreaTheme {
    /// Three looks for the ground, picked by tile so the speckle does not repeat in a grid.
    pub grass: [Spr; 3],
    pub trees: [Spr; 2],
    pub rock: Spr,
    /// The two frames of the river, and a tint for them.
    pub river: [Spr; 2],
    pub river_tint: u32,
    /// Chance that a free tile gets a tree, and then a rock.
    pub tree_rate: f64,
    pub rock_rate: f64,
}

pub fn area_theme(area: u8) -> AreaTheme {
    match area {
        AREA_MARSHMALLOW_MEADOWS => AreaTheme {
            grass: [Spr::OwGrassMeadow, Spr::OwGrassMeadowB, Spr::OwGrassMeadowC],
            trees: [Spr::OwPuff0, Spr::OwPuff1],
            rock: Spr::OwPuffRock,
            river: [Spr::OwRiver0, Spr::OwRiver1],
            river_tint: 0xffddee,
            tree_rate: 0.14,
            rock_rate: 0.03,
        },
        AREA_ROCK_CANDY_REACH => AreaTheme {
            grass: [Spr::OwGrassCandy, Spr::OwGrassCandyB, Spr::OwGrassCandyC],
            trees: [Spr::OwCandy0, Spr::OwCandy1],
            rock: Spr::OwCandyRock,
            river: [Spr::OwRiver0, Spr::OwRiver1],
            river_tint: 0xddeeff,
            tree_rate: 0.17,
            rock_rate: 0.03,
        },
        AREA_FROSTING_FRONTIER => AreaTheme {
            grass: [Spr::OwGrassFrost, Spr::OwGrassFrostB, Spr::OwGrassFrostC],
            trees: [Spr::OwFrost0, Spr::OwFrost1],
            rock: Spr::OwCake,
            river: [Spr::OwRiver0, Spr::OwRiver1],
            river_tint: 0xffffff,
            tree_rate: 0.13,
            rock_rate: 0.04,
        },
        // The lake is chocolate too; the island keeps the standard grass.
        AREA_GUMDROP_ISLE => AreaTheme {
            grass: [Spr::OwGrass, Spr::OwGrassB, Spr::OwGrassC],
            trees: [Spr::OwTree0, Spr::OwTree1],
            rock: Spr::OwRock,
            river: [Spr::OwRiver0, Spr::OwRiver1],
            river_tint: 0xffffff,
            tree_rate: 0.0,
            rock_rate: 0.0,
        },
        _ => AreaTheme {
            grass: [Spr::OwGrass, Spr::OwGrassB, Spr::OwGrassC],
            trees: [Spr::OwTree0, Spr::OwTree1],
            rock: Spr::OwRock,
            river: [Spr::OwRiver0, Spr::OwRiver1],
            river_tint: 0xffffff,
            tree_rate: 0.16,
            rock_rate: 0.02,
        },
    }
}

/// The ambient light on an area of the overworld by day. The sun is the same everywhere and makes up the
/// rest of full brightness (`render.rs` gives it the colour `(1 - ambient) / z`), so flat ground is lit to
/// exactly its own colour and only the relief in the sprites moves with the sun. The ambient is a little
/// cool and the sun a little warm, tinted a touch for each area.
pub fn map_ambient(area: u8) -> [f32; 3] {
    match area {
        AREA_MARSHMALLOW_MEADOWS => [0.60, 0.60, 0.69],
        AREA_ROCK_CANDY_REACH => [0.58, 0.64, 0.70],
        AREA_FROSTING_FRONTIER => [0.62, 0.66, 0.75],
        AREA_GUMDROP_ISLE => [0.60, 0.62, 0.69],
        _ => [0.58, 0.62, 0.68],
    }
}

/// A light on the overworld: where, how high, how far it reaches, its colour, and a slow pulse in hertz
/// (0 for steady).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MapLight {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub r: f64,
    pub c: [f64; 3],
    pub pulse: f64,
}

/// The overworld's lights: a lantern at each level entrance, a beacon on the saucer, a pulse on each
/// teleporter that is on (`met` says which are), a glow at the fountain and in the shop windows. They are
/// few enough that the 16 the renderer takes always cover every one in view (a test sweeps the camera).
pub fn map_lights(map: &TileMap, points: &[MapPoint], met: impl Fn(u16) -> bool) -> Vec<MapLight> {
    let mut out = Vec::new();
    for p in points {
        let (x, y) = (p.x + 0.5, p.y + 0.5);
        match p.kind {
            PtKind::Saucer => out.push(MapLight {
                x,
                y,
                z: 1.0,
                r: 3.5,
                c: [1.0, 0.85, 0.45],
                pulse: 0.0,
            }),
            PtKind::Level if !p.hidden || met(p.req) => out.push(MapLight {
                x,
                y,
                z: 0.9,
                r: 2.6,
                c: [1.0, 0.65, 0.3],
                pulse: 0.0,
            }),
            // The hidden pad has `met` false until the secret is found, so it lights only once it shows.
            PtKind::Tele if met(p.req) => out.push(MapLight {
                x,
                y,
                z: 0.8,
                r: 3.0,
                c: [0.3, 1.0, 1.1],
                pulse: 1.0,
            }),
            _ => {}
        }
    }
    for x in 0..map.w {
        for y in 0..map.h {
            let (c, r) = match map.get(x, y) {
                FOUNTAIN => ([0.4, 0.8, 1.0], 2.5),
                SHOP => ([1.0, 0.8, 0.4], 2.0),
                _ => continue,
            };
            out.push(MapLight {
                x: f64::from(x) + 0.5,
                y: f64::from(y) + 0.5,
                z: 0.8,
                r,
                c,
                pulse: 0.0,
            });
        }
    }
    out
}

pub struct MapData {
    pub map: TileMap,
    pub points: Vec<MapPoint>,
    pub areas: Vec<AreaRect>,
    pub start: (f64, f64),
}

impl MapData {
    /// The area that covers a tile; the first matching rectangle wins, so the lake is listed first.
    pub fn area_at(&self, x: i32, y: i32) -> u8 {
        area_at(&self.areas, x, y)
    }
}

pub fn area_at(areas: &[AreaRect], x: i32, y: i32) -> u8 {
    areas
        .iter()
        .find(|r| x >= r.x0 && x <= r.x1 && y >= r.y0 && y <= r.y1)
        .map_or(AREA_CRATER_FIELDS, |r| r.area)
}

/// Overworld areas: the lake and island first, then the four regions the rivers cut out.
pub fn overworld_areas() -> Vec<AreaRect> {
    let rect = |x0, y0, x1, y1, area| AreaRect {
        x0,
        y0,
        x1,
        y1,
        area,
    };
    vec![
        rect(20, 18, 29, 27, AREA_GUMDROP_ISLE),
        // The regions run out to the rivers between them, so a river takes its theme from the
        // region on its west or south side.
        rect(0, 0, 24, 22, AREA_CRATER_FIELDS),
        rect(0, 23, 24, 43, AREA_MARSHMALLOW_MEADOWS),
        rect(25, 0, 59, 22, AREA_FROSTING_FRONTIER),
        rect(25, 23, 59, 43, AREA_ROCK_CANDY_REACH),
    ]
}

/// A cell of the overworld thumbnail is `kind | aux << 4`: grass (aux is the area), river, or a
/// level node (aux is the level id).
pub const THUMB_GRASS: u8 = 0;
pub const THUMB_RIVER: u8 = 1;
pub const THUMB_NODE: u8 = 2;

/// The overworld flattened to one byte per tile, row 0 at the south, for the save-slot thumbnails.
/// Returns the width, the height and the cells. Nodes mark every level point; teleporters, signs and
/// the saucer are left out.
pub fn overworld_thumb() -> (i32, i32, Vec<u8>) {
    let data = build_overworld();
    let (w, h) = (data.map.w, data.map.h);
    let mut cells = vec![0u8; (w * h) as usize];
    for y in 0..h {
        for x in 0..w {
            let kind = if data.map.get(x, y) == RIVER {
                THUMB_RIVER
            } else {
                THUMB_GRASS
            };
            cells[(y * w + x) as usize] = kind | (data.area_at(x, y) << 4);
        }
    }
    for p in data
        .points
        .iter()
        .filter(|p| p.kind == PtKind::Level && !p.hidden)
    {
        let (x, y) = (p.x as i32, p.y as i32);
        if x >= 0 && y >= 0 && x < w && y < h {
            cells[(y * w + x) as usize] = THUMB_NODE | (p.level << 4);
        }
    }
    (w, h, cells)
}

pub fn build_overworld() -> MapData {
    let (w, h) = (60, 44);
    let mut map = TileMap::new(w, h, props());
    map.floor_solid = true;
    map.data.fill(GRASS);
    for x in 0..w {
        for y in 0..h {
            if x < 2 || y < 2 || x >= w - 2 || y >= h - 2 {
                map.set(x, y, RIVER);
            }
        }
    }
    // Two rivers cross the map and cut it into four regions; where they meet is a lake with an
    // island that can only be reached by teleporter.
    for y in 0..h {
        map.set(24, y, RIVER);
        map.set(25, y, RIVER);
    }
    for x in 0..w {
        map.set(x, 22, RIVER);
        map.set(x, 23, RIVER);
    }
    for x in 20..=29 {
        for y in 18..=27 {
            map.set(x, y, RIVER);
        }
    }
    for x in 23..=26 {
        for y in 21..=24 {
            map.set(x, y, GRASS);
        }
    }
    let req = |id: u8| level_bit(id) as u16;
    let pt = |kind, level, x, y, to, req, big| MapPoint {
        kind,
        level,
        x: f64::from(x),
        y: f64::from(y),
        to,
        req,
        big,
        hidden: false,
    };
    let secret = crate::world::SECRET_FOUND as u16;
    // Indices matter: each teleporter names its partner by index.
    let points = vec![
        pt(PtKind::Saucer, SAUCER, 8, 10, 0, 0, false),
        pt(PtKind::Level, CRATER, 14, 6, 0, 0, false),
        // The first area is a tutorial: Crater Fields, then Meteor Mesa, then Zarg Lookout.
        pt(PtKind::Level, METEOR_MESA, 16, 14, 0, req(CRATER), false),
        pt(
            PtKind::Level,
            ZARG_LOOKOUT,
            6,
            16,
            0,
            req(METEOR_MESA),
            false,
        ),
        // The three levels of Crater Fields power the way down to the Meadows.
        pt(
            PtKind::Tele,
            0,
            12,
            19,
            5,
            req(CRATER) | req(METEOR_MESA) | req(ZARG_LOOKOUT),
            false,
        ),
        pt(
            PtKind::Tele,
            0,
            12,
            26,
            4,
            req(CRATER) | req(METEOR_MESA) | req(ZARG_LOOKOUT),
            false,
        ),
        pt(PtKind::Level, MARSHMALLOW_MEADOWS, 5, 30, 0, 0, false),
        pt(PtKind::Level, FUDGE_BOG, 16, 31, 0, 0, false),
        pt(PtKind::Level, BONBON_PLAYHOUSE, 8, 37, 0, 0, false),
        pt(
            PtKind::Tele,
            0,
            20,
            38,
            10,
            req(MARSHMALLOW_MEADOWS) | req(BONBON_PLAYHOUSE),
            false,
        ),
        pt(
            PtKind::Tele,
            0,
            29,
            38,
            9,
            req(MARSHMALLOW_MEADOWS) | req(BONBON_PLAYHOUSE),
            false,
        ),
        pt(PtKind::Level, CAVES, 37, 34, 0, 0, false),
        pt(PtKind::Level, MIRROR_SHAFTS, 46, 38, 0, 0, false),
        pt(PtKind::Level, SUGAR_GLASS_GALLERY, 51, 30, 0, 0, false),
        pt(
            PtKind::Tele,
            0,
            53,
            27,
            15,
            req(CAVES) | req(MIRROR_SHAFTS),
            false,
        ),
        pt(
            PtKind::Tele,
            0,
            53,
            18,
            14,
            req(CAVES) | req(MIRROR_SHAFTS),
            false,
        ),
        pt(PtKind::Level, FROSTING_FLATS, 47, 14, 0, 0, false),
        pt(PtKind::Level, FROSTING_SPIRE, 33, 15, 0, 0, false),
        pt(PtKind::Level, COCOA_FOUNDRY, 34, 6, 0, 0, false),
        // The Citadel stays locked until both Frosting Frontier levels on the way are done.
        pt(
            PtKind::Level,
            CITADEL,
            44,
            6,
            0,
            req(FROSTING_SPIRE) | req(COCOA_FOUNDRY),
            true,
        ),
        // The secret pair: a pad on the island, always on show, and its hidden partner in a ring
        // of trees in the far north-east, which appears once the mural has been found.
        pt(PtKind::Tele, 0, 26, 22, 21, secret, false),
        MapPoint {
            hidden: true,
            ..pt(PtKind::Tele, 0, 55, 40, 20, secret, false)
        },
        pt(PtKind::Level, GUMDROP_ISLE, 24, 23, 0, 0, false),
        pt(PtKind::Level, WHISPER_HOLLOW, 41, 28, 0, 0, false),
        // Crater Corners, the little town around Zarg Lookout: the welcome sign and the bakery sign on
        // the street east of the tower, and the fountain sign at the fountain west of it.
        pt(PtKind::Sign, 0, 8, 16, 0, 0, false),
        pt(PtKind::Sign, 1, 11, 16, 0, 0, false),
        pt(PtKind::Sign, 2, 4, 16, 0, 0, false),
    ];
    let mut path = |x0: i32, y0: i32, x1: i32, y1: i32| {
        let (sx, sy) = ((x1 - x0).signum(), (y1 - y0).signum());
        if sx != 0 {
            let mut x = x0;
            while x != x1 + sx {
                map.set(x, y0, PATH);
                x += sx;
            }
        }
        if sy != 0 {
            let mut y = y0;
            while y != y1 + sy {
                map.set(x1, y, PATH);
                y += sy;
            }
        }
        map.set(x1, y1, PATH);
    };
    path(8, 9, 14, 6);
    path(14, 6, 16, 14);
    path(16, 14, 6, 16);
    path(6, 16, 12, 19);
    path(12, 26, 5, 30);
    path(5, 30, 8, 37);
    path(5, 30, 16, 31);
    path(8, 37, 20, 38);
    path(29, 38, 37, 34);
    path(37, 34, 46, 38);
    path(37, 34, 41, 28);
    path(46, 38, 53, 27);
    path(46, 38, 51, 30);
    path(53, 18, 33, 15);
    path(47, 18, 47, 14);
    path(33, 15, 34, 6);
    path(34, 6, 44, 6);
    let areas = overworld_areas();
    // Each tile draws its own roll from a hash of where it is, so moving a point or a path changes the
    // trees and rocks near it and nowhere else. The offsets keep the roll apart from the one the renderer
    // uses to pick a tree's look (`hashf(x, y)`), or every tree would pick the same look.
    let r = |x: i32, y: i32| hashf(x.wrapping_add(7919), y.wrapping_add(104_729));
    for x in 2..w - 2 {
        for y in 2..h - 2 {
            if map.get(x, y) != GRASS {
                continue;
            }
            let mut near = points
                .iter()
                .any(|p| (p.x - f64::from(x)).abs() < 3.0 && (p.y - f64::from(y)).abs() < 3.0);
            for dx in -1..=1 {
                for dy in -1..=1 {
                    let t = if map.in_bounds(x + dx, y + dy) {
                        map.get(x + dx, y + dy)
                    } else {
                        RIVER
                    };
                    if t == PATH {
                        near = true;
                    }
                }
            }
            if !near {
                let theme = area_theme(area_at(&areas, x, y));
                let q = r(x, y);
                if q < theme.tree_rate {
                    map.set(x, y, TREE);
                } else if q < theme.tree_rate + theme.rock_rate {
                    map.set(x, y, ROCK);
                }
            }
        }
    }
    // Crater Corners, around Zarg Lookout (6,16): a row of two houses and a shop north of the street that
    // runs east from the tower (a building is drawn standing up, so each keeps a free tile above it), and a
    // house and the fountain west of the tower. The street and the paths are untouched.
    for (x, y, t) in [
        (7, 17, HOUSE_A),
        (9, 17, HOUSE_B),
        (11, 17, SHOP),
        (4, 17, HOUSE_B),
        (4, 15, FOUNTAIN),
    ] {
        map.set(x, y, t);
    }
    // The hidden pad's clearing: a ring of trees two tiles out with a single gap to the west.
    for dx in -2i32..=2 {
        for dy in -2i32..=2 {
            let (x, y) = (55 + dx, 40 + dy);
            let ring = dx.abs() == 2 || dy.abs() == 2;
            if ring && !(dx == -2 && dy == 0) {
                map.set(x, y, TREE);
            } else if map.get(x, y) != GRASS {
                map.set(x, y, GRASS);
            }
        }
    }
    // Keep the way in clear: the gap must lead out onto open ground.
    for x in 50..=53 {
        if map.get(x, 40) != PATH {
            map.set(x, 40, GRASS);
        }
    }
    MapData {
        map,
        points,
        areas,
        start: (9.2, 8.2),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count(m: &TileMap, t: u8) -> usize {
        m.data.iter().filter(|&&v| v == t).count()
    }

    #[test]
    fn every_level_has_a_free_start_and_the_right_features() {
        for id in 0..LEVEL_COUNT {
            let l = build_level(id);
            let (sx, sy) = l.start;
            for dy in 0..2 {
                assert!(
                    !l.map.solid(sx as i32, sy as i32 + dy, false, 0.0),
                    "level {id} start blocked"
                );
            }
            assert!(
                l.map.solid(sx as i32, sy as i32 - 1, false, 0.0),
                "level {id} start floating"
            );
            assert!(count(&l.map, CRYS) > 0);
            assert!(!l.items.is_empty());
            for it in &l.items {
                assert!(
                    !l.map
                        .solid(it.x.floor() as i32, it.y.floor() as i32, false, 0.0),
                    "level {id}: {:?} at {:.1},{:.1} is inside solid ground",
                    it.kind,
                    it.x,
                    it.y
                );
            }
            assert!(!l.spawns.is_empty());
        }
        assert_eq!(count(&build_level(CRATER).map, EXIT), 2);
        assert_eq!(count(&build_level(CAVES).map, EXIT), 2);
        assert!(build_level(CITADEL).arena.is_some());
        assert_eq!(build_level(CAVES).plats.len(), 3);
    }

    #[test]
    fn the_level_table_is_indexed_by_id_and_every_builder_agrees() {
        assert_eq!(LEVELS.len(), usize::from(TABLE_LEN));
        for (i, def) in LEVELS.iter().enumerate() {
            assert_eq!(usize::from(def.id), i, "table slot {i} holds id {}", def.id);
            let l = (def.build)();
            assert_eq!(l.id, def.id);
            assert_eq!(l.theme, def.theme);
            assert_eq!(l.area, def.area);
        }
        assert_eq!(
            build_level(200).id,
            0,
            "unknown ids fall back to the first level"
        );
    }

    /// Widest run of columns with nothing to stand on, ignoring columns a hover platform spans.
    fn widest_unsupported_gap(l: &LevelData) -> i32 {
        let (mut widest, mut run) = (0, 0);
        for x in 1..l.map.w - 1 {
            let carried = l.plats.iter().any(|p| {
                f64::from(x) + 1.0 > p.ax.min(p.bx) && f64::from(x) < p.ax.max(p.bx) + p.w
            });
            let supported = (0..l.map.h - 1).any(|y| {
                let t = l.map.get(x, y);
                let top = l.map.props.flags(t) != 0 || l.map.is_slope(t);
                top && t != CHOC && l.map.get(x, y + 1) == EMPTY
            });
            if supported || carried {
                run = 0;
            } else {
                run += 1;
                widest = widest.max(run);
            }
        }
        widest
    }

    #[test]
    fn meteor_mesa_has_no_gap_wider_than_a_comfortable_jump() {
        let l = build_level(METEOR_MESA);
        assert!(
            widest_unsupported_gap(&l) <= 3,
            "gap of {} columns",
            widest_unsupported_gap(&l)
        );
        assert_eq!(count(&l.map, EXIT), 2);
        assert!(l.arena.is_none() && l.plats.len() == 1);
        assert_eq!(l.theme, Theme::OpenSky);
    }

    /// Whether a bubble fired from anywhere Ben can stand or jump to reaches a crystal switch
    /// before a mirror or a wall turns or stops it: straight up from the ground, or sideways at any
    /// height of an ordinary jump (3.4 tiles; the pogo goes higher and is not guarded). A bubble
    /// flies about 17 tiles. Returns where the shot started and which switch it hit.
    fn switch_in_straight_line(l: &LevelData) -> Option<(f64, f64, f64, f64)> {
        let cells = |kinds: &[Kind]| -> Vec<(f64, f64)> {
            l.spawns
                .iter()
                .filter(|s| kinds.contains(&s.kind))
                .map(|s| (s.x + 0.5, s.y + 0.5))
                .collect()
        };
        let switches = cells(&[Kind::CrystalSwitch]);
        let mirrors = cells(&[Kind::Mirror, Kind::Swivel]);
        // Follows a bubble from (x, y) in direction (dx, dy) and returns the switch it hits, if
        // nothing solid or a mirror gets in the way first.
        let ray = |x: f64, y: f64, dx: f64, dy: f64| -> Option<(f64, f64)> {
            let (mut px, mut py) = (x, y);
            for _ in 0..352 {
                if l.map
                    .solid(px.floor() as i32, py.floor() as i32, false, 0.0)
                {
                    return None;
                }
                if mirrors.iter().any(|m| {
                    (m.0 - px).abs() < crate::world::MIRROR_REACH
                        && (m.1 - py).abs() < crate::world::MIRROR_REACH
                }) {
                    return None;
                }
                if let Some(s) = switches.iter().find(|s| {
                    (s.0 - px).abs() < crate::world::SWITCH_REACH
                        && (s.1 - py).abs() < crate::world::SWITCH_REACH
                }) {
                    return Some(*s);
                }
                px += dx * 0.05;
                py += dy * 0.05;
            }
            None
        };
        for x in 0..l.map.w {
            for y in 0..l.map.h - 2 {
                let t = l.map.get(x, y);
                let floor = t == PLAT || l.map.is_solid_tile(t);
                // He needs room to stand: the two tiles above must be free (he is 1.4 tall).
                let free = |yy: i32| matches!(l.map.get(x, yy), EMPTY | WALLBG);
                if !floor || !free(y + 1) || !free(y + 2) {
                    continue;
                }
                let stand = f64::from(y + 1);
                // Ben's centre can be anywhere that leaves part of his 0.7-wide body on the tile.
                let mut cx = f64::from(x) - 0.3;
                while cx < f64::from(x) + 1.3 {
                    if let Some(s) = ray(cx, stand + 1.5, 0.0, 1.0) {
                        return Some((cx, stand + 1.5, s.0, s.1));
                    }
                    // Sideways, at every height of a jump from this spot (a shot leaves 0.6 tiles
                    // in front of him, at 0.85 above his feet).
                    let mut feet = stand;
                    while feet <= stand + 3.4 {
                        // His body must fit in open space at this height.
                        let fits = (((cx - 0.35).floor() as i32)..=((cx + 0.34).floor() as i32))
                            .all(|bx| {
                                (feet.floor() as i32..=(feet + 1.39).floor() as i32)
                                    .all(|by| !l.map.is_solid_tile(l.map.get(bx, by)))
                            });
                        if !fits {
                            break;
                        }
                        for dir in [-1.0, 1.0] {
                            if let Some(s) = ray(cx + dir * 0.6, feet + 0.85, dir, 0.0) {
                                return Some((cx, feet + 0.85, s.0, s.1));
                            }
                        }
                        feet += 0.1;
                    }
                    cx += 0.05;
                }
            }
        }
        None
    }

    #[test]
    fn no_crystal_switch_can_be_shot_directly_from_a_ledge() {
        let l = build_level(MIRROR_SHAFTS);
        assert_eq!(
            switch_in_straight_line(&l),
            None,
            "a switch can be shot directly from somewhere Ben can stand or jump: (x, y, switch)"
        );
    }

    #[test]
    fn no_press_sweeps_through_a_ledge_or_solid_ground() {
        for def in LEVELS.iter() {
            let l = (def.build)();
            for s in l.spawns.iter().filter(|s| s.kind == Kind::Press) {
                // A press is 2 wide (body x = spawn x - 0.5), raised with its bottom at `s.y`, and it
                // drops 3 tiles; its box sweeps from the floor up to its raised top.
                let (x0, x1) = ((s.x - 0.5).floor() as i32, (s.x + 1.4).floor() as i32);
                let (y0, y1) = ((s.y - 3.0).floor() as i32, (s.y + 1.4).floor() as i32);
                for x in x0..=x1 {
                    // The floor itself is the row below the press's lowest point.
                    for y in y0..=y1 {
                        let t = l.map.get(x, y);
                        assert!(
                            !l.map.is_solid_tile(t) && t != PLAT,
                            "level {}: press at {} sweeps through tile {t} at {x},{y}",
                            def.id,
                            s.x
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn progress_bits_cover_every_level() {
        for id in 0..LEVEL_COUNT {
            assert_ne!(crate::world::PROGRESS_BITS & level_bit(id), 0);
        }
        assert_eq!(
            crate::world::PROGRESS_BITS & crate::world::SECRET_FOUND,
            crate::world::SECRET_FOUND
        );
    }

    #[test]
    fn doors_are_paired_with_keys() {
        let c = build_level(CRATER);
        assert!(c.items.iter().any(|i| i.kind == KeyRed));
        assert!(count(&c.map, DOOR_R) > 0);
        let v = build_level(CAVES);
        assert!(v.items.iter().any(|i| i.kind == KeyBlue));
        assert!(count(&v.map, DOOR_B) > 0);
    }

    /// A fingerprint of the overworld's tiles and where its points are, to notice an edit to the map.
    fn overworld_fingerprint() -> u32 {
        let m = build_overworld();
        let mut h = Fnv::new();
        m.map.data.iter().for_each(|&t| h.byte(t));
        for p in &m.points {
            h.f64(p.x);
            h.f64(p.y);
        }
        h.0
    }

    #[test]
    fn every_areas_ambient_light_is_readable_and_leaves_room_for_a_warm_sun() {
        let d = crate::render::map_sun_dir();
        for a in [
            AREA_CRATER_FIELDS,
            AREA_MARSHMALLOW_MEADOWS,
            AREA_ROCK_CANDY_REACH,
            AREA_FROSTING_FRONTIER,
            AREA_GUMDROP_ISLE,
        ] {
            let amb = map_ambient(a);
            let sun: Vec<f32> = amb.iter().map(|c| (1.0 - c) / d[2]).collect();
            for i in 0..3 {
                // Ground facing the screen gets exactly full brightness: ambient plus the sun's share.
                assert!(
                    (amb[i] + sun[i] * d[2] - 1.0).abs() < 1e-6,
                    "area {a} channel {i}"
                );
                assert!(
                    amb[i] >= 0.45,
                    "area {a}: the shade is too dark to read under the thumbs"
                );
                assert!(sun[i] >= 0.0, "area {a}: ambient above full brightness");
            }
            assert!(
                sun[0] > sun[2],
                "area {a}: the sun is warmer than the shade"
            );
        }
    }

    #[test]
    fn the_map_lights_fit_the_budget_wherever_the_camera_goes() {
        let m = build_overworld();
        // Every teleporter on and the hidden pad found: the most lights the map ever has.
        let lights = map_lights(&m.map, &m.points, |_| true);
        assert!(
            lights.len() >= 20,
            "level entrances, the saucer, pads and the town: {}",
            lights.len()
        );
        // The widest view the shell asks for is about 26 by 12 tiles; round it up.
        let (half_w, half_h) = (14.0, 7.0);
        let mut worst = 0;
        let mut x = half_w;
        while x <= f64::from(m.map.w) - half_w {
            let mut y = half_h;
            while y <= f64::from(m.map.h) - half_h {
                // A light matters when its reach touches the view.
                let reaching = lights
                    .iter()
                    .filter(|l| {
                        let dx = ((l.x - x).abs() - half_w).max(0.0);
                        let dy = ((l.y - y).abs() - half_h).max(0.0);
                        dx.hypot(dy) < l.r
                    })
                    .count();
                worst = worst.max(reaching);
                y += 1.0;
            }
            x += 1.0;
        }
        // The renderer takes the 16 nearest the camera; one spare means none ever pops in or out.
        assert!(worst <= 15, "{worst} lights reach one view");
    }

    #[test]
    fn overworld_tiles_are_pinned() {
        // A change to the map (a tile, a point) changes this on purpose: look at what moved, then update it.
        // Trees and rocks come from a hash of each tile's place, so an edit moves only the ones near it.
        assert_eq!(
            overworld_fingerprint(),
            0xb3e9_5796,
            "the overworld changed: update the pinned fingerprint if that was meant"
        );
    }

    #[test]
    fn crater_corners_stands_around_zarg_lookout() {
        let m = build_overworld();
        let lookout = m
            .points
            .iter()
            .find(|p| p.kind == PtKind::Level && p.level == ZARG_LOOKOUT)
            .expect("the watchtower is on the map");
        let area = area_at(&m.areas, lookout.x as i32, lookout.y as i32);
        let town = [HOUSE_A, HOUSE_B, SHOP, FOUNTAIN];
        let mut buildings = 0;
        for x in 0..m.map.w {
            for y in 0..m.map.h {
                if !town.contains(&m.map.get(x, y)) {
                    continue;
                }
                buildings += 1;
                let d = (f64::from(x) - lookout.x).hypot(f64::from(y) - lookout.y);
                assert!(
                    d <= 8.0,
                    "building at {x},{y} is {d:.1} tiles from the tower"
                );
                assert_eq!(
                    area_at(&m.areas, x, y),
                    area,
                    "building at {x},{y} is in Crater Fields"
                );
                assert!(
                    !m.map.solid(x, y + 1, false, 0.0) && m.map.get(x, y + 1) != PATH,
                    "a tile stays free above the building at {x},{y}, so it can stand up there"
                );
            }
        }
        assert_eq!(buildings, 5, "two houses, a shop, a house and a fountain");
        // Nothing of the old street is left east of the first level.
        assert_ne!(m.map.get(20, 6), PATH);
        assert!(!town.contains(&m.map.get(17, 4)) && !town.contains(&m.map.get(23, 6)));
    }

    #[test]
    fn the_three_signs_stand_in_the_town_and_keep_clear_of_every_other_point() {
        let m = build_overworld();
        let lookout = m
            .points
            .iter()
            .find(|p| p.kind == PtKind::Level && p.level == ZARG_LOOKOUT)
            .expect("the watchtower is on the map");
        let area = area_at(&m.areas, lookout.x as i32, lookout.y as i32);
        let signs: Vec<&MapPoint> = m.points.iter().filter(|p| p.kind == PtKind::Sign).collect();
        assert_eq!(signs.len(), 3);
        // The texts are by number: welcome, the bakery, the fountain.
        assert_eq!(signs.iter().map(|p| p.level).collect::<Vec<_>>(), [0, 1, 2]);
        for (i, s) in signs.iter().enumerate() {
            assert_eq!(
                area_at(&m.areas, s.x as i32, s.y as i32),
                area,
                "sign {i} is in Crater Fields"
            );
            for (j, o) in m.points.iter().enumerate() {
                if core::ptr::eq(*s, o) {
                    continue;
                }
                let d = (s.x - o.x).hypot(s.y - o.y);
                assert!(
                    d >= 2.0,
                    "sign {i} is {d:.1} tiles from point {j}, too close to tell apart"
                );
            }
        }
    }

    #[test]
    fn overworld_points_are_walkable_and_teleporters_pair_up() {
        let m = build_overworld();
        for (i, p) in m.points.iter().enumerate() {
            let t = m.map.get(p.x as i32, p.y as i32);
            assert!(
                t == PATH || t == GRASS,
                "point {i} at {},{} on tile {t}",
                p.x,
                p.y
            );
            if p.kind == PtKind::Tele {
                let q = &m.points[p.to];
                assert_eq!(q.kind, PtKind::Tele);
                assert_eq!(q.to, i, "teleporter {i} and {} pair up", p.to);
            }
        }
        assert_eq!(m.points.len(), 27);
    }

    /// Indices of the points a walker can reach from tile (x, y) without crossing anything solid.
    fn reachable_points(m: &MapData, x: i32, y: i32) -> Vec<usize> {
        let (w, h) = (m.map.w, m.map.h);
        let mut seen = vec![false; (w * h) as usize];
        let mut stack = vec![(x, y)];
        while let Some((cx, cy)) = stack.pop() {
            if !m.map.in_bounds(cx, cy) || seen[(cy * w + cx) as usize] {
                continue;
            }
            let t = m.map.get(cx, cy);
            if t != GRASS && t != PATH {
                continue;
            }
            seen[(cy * w + cx) as usize] = true;
            stack.extend([(cx + 1, cy), (cx - 1, cy), (cx, cy + 1), (cx, cy - 1)]);
        }
        m.points
            .iter()
            .enumerate()
            .filter(|(_, p)| seen[(p.y as i32 * w + p.x as i32) as usize])
            .map(|(i, _)| i)
            .collect()
    }

    #[test]
    fn each_region_connects_to_the_next_only_through_a_teleporter() {
        let m = build_overworld();
        let levels = |ids: &[usize]| -> Vec<u8> {
            let mut v: Vec<u8> = ids
                .iter()
                .filter(|&&i| m.points[i].kind == PtKind::Level)
                .map(|&i| m.points[i].level)
                .collect();
            v.sort_unstable();
            v
        };
        // From the saucer: Crater Fields only.
        let nw = reachable_points(&m, m.start.0 as i32, m.start.1 as i32);
        assert_eq!(levels(&nw), vec![CRATER, METEOR_MESA, ZARG_LOOKOUT]);
        // Each teleporter's far end opens onto the next region and not the previous one.
        let from = |i: usize| reachable_points(&m, m.points[i].x as i32, m.points[i].y as i32);
        let sw = levels(&from(5));
        assert_eq!(sw, vec![MARSHMALLOW_MEADOWS, BONBON_PLAYHOUSE, FUDGE_BOG]);
        let se = levels(&from(10));
        assert_eq!(
            se,
            vec![CAVES, MIRROR_SHAFTS, SUGAR_GLASS_GALLERY, WHISPER_HOLLOW]
        );
        let ne = levels(&from(15));
        assert_eq!(
            ne,
            vec![CITADEL, FROSTING_FLATS, FROSTING_SPIRE, COCOA_FOUNDRY]
        );
    }

    #[test]
    fn the_island_cannot_be_reached_on_foot_and_the_hidden_pad_can_be_walked_into() {
        let m = build_overworld();
        let island_pad = 20;
        let hidden_pad = 21;
        assert_eq!(
            (m.points[island_pad].x, m.points[island_pad].y),
            (26.0, 22.0)
        );
        let isle = m
            .points
            .iter()
            .position(|p| p.level == GUMDROP_ISLE && p.kind == PtKind::Level)
            .unwrap();
        for start in [0usize, 5, 10, 15] {
            let reach = reachable_points(&m, m.points[start].x as i32, m.points[start].y as i32);
            assert!(
                !reach.contains(&island_pad),
                "island pad reachable from {start}"
            );
            assert!(
                !reach.contains(&isle),
                "Gumdrop Isle reachable from {start}"
            );
        }
        // The island itself is walkable once there.
        let there = reachable_points(&m, 26, 22);
        assert!(there.contains(&isle) && there.contains(&island_pad));
        // The hidden pad sits in the Rock Candy Reach region and is reached through the ring's gap.
        assert!(m.points[hidden_pad].hidden);
        let se = reachable_points(&m, m.points[10].x as i32, m.points[10].y as i32);
        assert!(
            se.contains(&hidden_pad),
            "the ring has a gap a walker can use"
        );
        assert_eq!(m.map.get(53, 40), GRASS, "the gap");
        assert_eq!(
            m.map.get(57, 40),
            TREE,
            "the ring is closed on the far side"
        );
    }

    #[test]
    fn no_path_runs_over_a_river_and_every_tile_belongs_to_an_area() {
        let m = build_overworld();
        for x in 0..m.map.w {
            for y in 0..m.map.h {
                let on_river_line = x < 2
                    || y < 2
                    || x >= m.map.w - 2
                    || y >= m.map.h - 2
                    || (24..=25).contains(&x)
                    || (22..=23).contains(&y);
                let lake = (20..=29).contains(&x) && (18..=27).contains(&y);
                let island = (23..=26).contains(&x) && (21..=24).contains(&y);
                if (on_river_line || lake) && !island {
                    assert_ne!(m.map.get(x, y), PATH, "path on a river at {x},{y}");
                }
            }
        }
        assert_eq!(m.area_at(5, 5), AREA_CRATER_FIELDS);
        assert_eq!(m.area_at(5, 35), AREA_MARSHMALLOW_MEADOWS);
        assert_eq!(m.area_at(40, 35), AREA_ROCK_CANDY_REACH);
        assert_eq!(m.area_at(40, 5), AREA_FROSTING_FRONTIER);
        assert_eq!(m.area_at(24, 23), AREA_GUMDROP_ISLE);
    }

    #[test]
    fn every_level_sits_in_the_area_its_point_is_in() {
        let m = build_overworld();
        for p in m.points.iter().filter(|p| p.kind == PtKind::Level) {
            let area = m.area_at(p.x as i32, p.y as i32);
            assert_eq!(
                area,
                LEVELS[usize::from(p.level)].area,
                "level {} is in area {area} on the map",
                p.level
            );
        }
    }

    #[test]
    fn the_first_area_is_a_tutorial_in_order_and_the_rivers_take_a_theme() {
        let m = build_overworld();
        let find = |id: u8| {
            m.points
                .iter()
                .find(|p| p.kind == PtKind::Level && p.level == id)
                .unwrap()
        };
        assert_eq!(find(CRATER).req, 0, "the first level is open");
        assert_eq!(find(METEOR_MESA).req, level_bit(CRATER) as u16);
        assert_eq!(find(ZARG_LOOKOUT).req, level_bit(METEOR_MESA) as u16);
        // Every tile has an area, including the dividers, and a divider is not left to the default:
        // the river between the north-west and north-east regions belongs to one of them.
        assert_eq!(m.area_at(24, 30), AREA_MARSHMALLOW_MEADOWS);
        assert_eq!(m.area_at(25, 30), AREA_ROCK_CANDY_REACH);
        assert_eq!(m.area_at(40, 22), AREA_FROSTING_FRONTIER);
        assert_eq!(m.area_at(40, 23), AREA_ROCK_CANDY_REACH);
        assert_eq!(m.area_at(10, 22), AREA_CRATER_FIELDS);
        assert_eq!(m.area_at(10, 23), AREA_MARSHMALLOW_MEADOWS);
    }

    #[test]
    fn gates_follow_the_plan() {
        let m = build_overworld();
        let level = |id: u8| {
            m.points
                .iter()
                .find(|p| p.kind == PtKind::Level && p.level == id)
                .unwrap()
        };
        assert_eq!(
            level(CITADEL).req,
            (level_bit(FROSTING_SPIRE) | level_bit(COCOA_FOUNDRY)) as u16
        );
        assert_eq!(
            m.points[4].req,
            (level_bit(CRATER) | level_bit(METEOR_MESA) | level_bit(ZARG_LOOKOUT)) as u16
        );
        assert_eq!(
            m.points[9].req,
            (level_bit(MARSHMALLOW_MEADOWS) | level_bit(BONBON_PLAYHOUSE)) as u16
        );
        assert_eq!(
            m.points[14].req,
            (level_bit(CAVES) | level_bit(MIRROR_SHAFTS)) as u16
        );
        assert_eq!(m.points[20].req, crate::world::SECRET_FOUND as u16);
        assert_eq!(m.points[21].req, crate::world::SECRET_FOUND as u16);
    }
}
