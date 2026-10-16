// Episode 1 world state and the fixed-step level, overworld, player and enemy logic.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! The Episode 1 world: game state, level and overworld simulation, player and enemy logic.
//! A port of the prototype's `engine.js` simulation half, with f64 state and a deterministic
//! fixed 60 Hz step.

#[allow(unused_imports)]
use alloc::{boxed::Box, vec, vec::Vec};
#[allow(unused_imports)]
use lf_sim::fmath::{FloatMath, FloatMath32};
use crate::ents::*;
use crate::levels::{self, Arena, MapPoint, PtKind, Room};
use crate::render::{CamMode, Theme};
use crate::sprites::Spr;
use crate::text::{ev, Cap, Toast};
use crate::tiles::*;
use lf_sim::tilemap::ONEWAY;
use lf_sim::{
    hashf, Body, EventQueue, InstanceBuffer, LightPool, Platform, Rng, SpriteRect, TileMap,
    GRAVITY, STEP,
};

pub const MAX_INSTANCES: usize = 120_000;

/// How long after Ben is back on the map a press is ignored. The press that closed the end card or the
/// game over card may still be down, and it must not open the level straight away: it takes a new one.
pub const MAP_ARM_SECS: f64 = 0.4;
/// How long the flag takes to fly from Ben to the pole of a level cleared for the first time.
pub const FLAG_THROW_SECS: f64 = 0.9;

/// Whether a 0.6-tile body at `(x, y)` on the map stands clear of rivers, trees, rocks and buildings, and
/// inside the map.
fn map_body_free(map: &TileMap, (x, y): (f64, f64)) -> bool {
    // Off the map entirely (a hand-edited save) is no better than inside a rock.
    x >= 0.0
        && y >= 0.0
        && x + 0.6 < f64::from(map.w)
        && y + 0.6 < f64::from(map.h)
        && [(0.0, 0.0), (0.6, 0.0), (0.0, 0.6), (0.6, 0.6)]
            .iter()
            .all(|(dx, dy)| {
                !map.solid((x + dx).floor() as i32, (y + dy).floor() as i32, false, 0.0)
            })
}

/// How far a press must have dropped from its raised position before it can crush Ben: its
/// bottom is then within about a tile of the floor.
pub const PRESS_DANGER_DROP: f64 = 1.6;

/// How close a bubble must pass to a mirror's centre to bounce off it, and to a crystal switch to
/// open its gate. A mirror is the larger target: a bubble Ben fires from just beside one still
/// meets it, so he cannot slip a shot past a mirror by standing against it.
pub const MIRROR_REACH: f64 = 0.8;
pub const SWITCH_REACH: f64 = 0.55;

/// Enemies in a hidden room do not move while its front wall is more opaque than this.
pub const ROOM_DORMANT_ALPHA: f64 = 0.95;

/// How opaque a room's front wall is while Ben stands inside it.
pub const ROOM_SEEN_ALPHA: f64 = 0.22;

/// Vine climbing speed up and down, and across, in tiles per second.
pub const VINE_SPEED: f64 = 3.5;
pub const VINE_SIDE_SPEED: f64 = 3.0;

/// How high above his feet a wall's top edge can be for Ben to pull himself up onto it.
pub const MANTLE_REACH: f64 = 0.95;

/// How long a mantle takes from the grab to standing on the ledge, in seconds. This is the one knob
/// for the pull-up: the sprite frames and the path both follow its progress.
pub const MANTLE_TIME: f64 = 0.30;

/// The share of `MANTLE_TIME` spent hauling straight up the wall; the rest steps over the lip.
pub const MANTLE_RISE_SHARE: f64 = 0.6;

/// A kick off a wall: how fast it sends Ben up and away, and how long before the next one.
pub const WALL_KICK_UP: f64 = 19.0;
pub const WALL_KICK_AWAY: f64 = 6.0;
pub const WALL_KICK_COOLDOWN: f64 = 0.25;

/// How fast Ben must be rising (tiles per second) when his head meets a ceiling for it to clunk.
pub const HEAD_BUMP_MIN_SPEED: f64 = 1.0;

/// How far Ben must have risen above his last footing when he meets the ceiling, so a bounce with no room to rise is silent.
pub const HEAD_BUMP_MIN_RISE: f64 = 0.05;

/// How tall Ben is drawn on the pogo stick (32 pixels), against his 1.4-tile body.
pub const POGO_DRAWN_HEIGHT: f64 = 2.0;

/// Whether a rising pogo bounce stops with the drawn head, not just the body, against a ceiling. Switch
/// off to give the pogo the body's 1.4-tile height again.
pub const POGO_HEAD_BOX: bool = true;

/// Ladder climbing speed in tiles per second.
pub const CLIMB_SPEED: f64 = 4.5;

/// How far from its home floor (in tiles) an enemy may be, and for how long, before it is sent home.
/// The levels attract mode shows in turn.
pub const ATTRACT_LEVELS: [u8; 3] = [levels::CRATER, levels::CAVES, levels::CITADEL];
/// Attract camera speed in tiles per second, and the longest stretch it pans across in tiles.
const ATTRACT_SPEED: f64 = 2.2;
const ATTRACT_SPAN: f64 = 46.0;
pub const STRAY_HEIGHT: f64 = 4.0;
pub const STRAY_SECS: f64 = 3.0;

/// Input bits passed to `step`.
pub mod input {
    pub const LEFT: u32 = 1;
    pub const RIGHT: u32 = 2;
    pub const UP: u32 = 4;
    pub const DOWN: u32 = 8;
    pub const JUMP: u32 = 16;
    pub const POGO: u32 = 32;
    pub const FIRE: u32 = 64;
    pub const CONFIRM: u32 = 128;
}
use input::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Nothing simulated (cinematics are drawn by the shell).
    None,
    /// A level scrolling by behind the title screen.
    Attract,
    Map,
    Level,
}

/// Progress that survives across levels (and is saved).
#[derive(Clone, Debug)]
pub struct Game {
    pub lives: i32,
    pub score: i32,
    pub next_life: i32,
    pub ammo: i32,
    /// Cleared levels (`levels::level_bit`) plus flags such as `SECRET_FOUND`.
    pub done: u32,
    pub map_pos: Option<(f64, f64)>,
}

/// Bit in `Game::done` set once the secret teleporter has been discovered.
pub const SECRET_FOUND: u32 = 1 << 15;

/// Every bit of `Game::done` that carries meaning; anything else is dropped when loading a save.
pub const PROGRESS_BITS: u32 = ((1 << levels::LEVEL_COUNT) - 1) | SECRET_FOUND;

impl Game {
    /// Takes a saved cleared-levels mask, dropping every bit that means nothing.
    pub fn load_done(&mut self, mask: u32) {
        self.done = mask & PROGRESS_BITS;
    }

    pub fn is_done(&self, level: u8) -> bool {
        level < levels::LEVEL_COUNT && self.done & levels::level_bit(level) != 0
    }

    pub fn set_done(&mut self, level: u8) {
        // The saucer has no goal and no bit of its own (its id would land on the secret flag).
        if level < levels::LEVEL_COUNT {
            self.done |= levels::level_bit(level);
        }
    }

    pub fn fresh() -> Self {
        Game {
            lives: 3,
            score: 0,
            next_life: 100,
            ammo: 5,
            done: 0,
            map_pos: None,
        }
    }
}

/// A pull-up onto a ledge in progress. While it runs Ben is carried from `from` to `to` along a
/// rise-then-step path and nothing else moves him.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mantle {
    /// Seconds since the grab.
    pub t: f64,
    pub from: (f64, f64),
    pub to: (f64, f64),
}

impl Mantle {
    /// How far through the pull-up he is, from 0 at the grab to 1 on the ledge.
    pub fn progress(&self) -> f64 {
        (self.t / MANTLE_TIME).clamp(0.0, 1.0)
    }

    /// Where Ben's feet are after `t` seconds: straight up the wall first, then over the lip.
    pub fn at(&self, t: f64) -> (f64, f64) {
        let ease = |k: f64| {
            let k = k.clamp(0.0, 1.0);
            k * k * (3.0 - 2.0 * k)
        };
        let u = (t / MANTLE_TIME).clamp(0.0, 1.0);
        if u >= 1.0 {
            self.to
        } else if u < MANTLE_RISE_SHARE {
            let k = ease(u / MANTLE_RISE_SHARE);
            (self.from.0, self.from.1 + (self.to.1 - self.from.1) * k)
        } else {
            let k = ease((u - MANTLE_RISE_SHARE) / (1.0 - MANTLE_RISE_SHARE));
            (self.from.0 + (self.to.0 - self.from.0) * k, self.to.1)
        }
    }
}

/// How Ben died, as read through `state_get(DEATH_CAUSE)`. The numbers are part of the shell's protocol.
pub mod death {
    pub const FELL: u8 = 1;
    pub const SPIKES: u8 = 2;
    pub const CHOCOLATE: u8 = 3;
    pub const MOLTEN_METAL: u8 = 4;
    pub const ENEMY: u8 = 5;
    pub const PUFF: u8 = 6;
    pub const SHOT: u8 = 7;
}

#[derive(Clone, Debug)]
pub struct Player {
    pub b: Body,
    pub face: f64,
    pub pogo: bool,
    pub cut: bool,
    pub anim: f64,
    pub shoot_t: f64,
    /// Where the last shot went: 1 up, -1 down, 0 sideways.
    pub aim: i8,
    /// 0 while alive, otherwise seconds since death.
    pub dead: f64,
    pub dead_sent: bool,
    /// How Ben died, one of the `death` constants (0 while alive or when the cause is not known). Only read
    /// by the shell, never by a rule.
    pub death_cause: u8,
    /// What it was: the tile id for spikes, chocolate and molten metal, the `ents::Kind` for an enemy, else 0.
    pub death_detail: u16,
    pub inv: f64,
    pub squash: f64,
    /// Whether this rise has already clunked, so one bounce or jump sounds once.
    pub bumped: bool,
    /// Where Ben's current rise began (his height at the last tick he was not rising), for telling a
    /// real rise from a bounce with no room.
    pub rise_y: f64,
    pub look_down: f64,
    pub look_up: f64,
    /// Holding a ladder: gravity and running are off and Up/Down move Ben along it.
    pub climb: bool,
    /// Set when Ben jumps off a ladder: he cannot grab one again until he lets go of Up and Down or
    /// leaves the ladder, so jumping with Up held does not snap him straight back onto it.
    pub no_grab: bool,
    /// Climbing a vine rather than a ladder: free movement in every direction.
    pub wall: bool,
    /// Seconds before Ben can kick off a wall again.
    pub wall_t: f64,
    /// Which wall he last kicked off (-1 left, 1 right, 0 none yet). He cannot kick the same side
    /// twice running, so a single wall cannot be climbed; only two walls facing each other can.
    pub kick_side: f64,
    pub rot: f64,
    pub hidden: bool,
    /// Set while Ben pulls himself up onto a ledge; see `Mantle`.
    pub mantle: Option<Mantle>,
}

impl Player {
    pub fn level(x: f64, y: f64) -> Self {
        Player {
            b: Body::new(x, y, 0.7, 1.4),
            face: 1.0,
            pogo: false,
            cut: false,
            anim: 0.0,
            shoot_t: 0.0,
            aim: 0,
            dead: 0.0,
            dead_sent: false,
            death_cause: 0,
            death_detail: 0,
            inv: 0.0,
            squash: 0.0,
            bumped: false,
            rise_y: 0.0,
            look_down: 0.0,
            look_up: 0.0,
            climb: false,
            no_grab: false,
            wall: false,
            wall_t: 0.0,
            kick_side: 0.0,
            rot: 0.0,
            hidden: false,
            mantle: None,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct StaticLight {
    pub x: f64,
    pub y: f64,
    pub c: [f64; 3],
}

#[derive(Clone, Copy, Debug)]
pub struct Hazard {
    pub x: f64,
    pub y: f64,
    pub w: f64,
}

/// Counters behind the small jokes: each fires once per level visit.
#[derive(Clone, Copy, Default)]
pub struct Fun {
    pub pogos: u32,
    pub marsh: u32,
    pub kicks: u32,
    pub idle: f64,
    pub told: u8,
}

pub const JOKE_POGO: u8 = 1;
pub const JOKE_MARSH: u8 = 2;
pub const JOKE_KICK: u8 = 4;
pub const JOKE_IDLE: u8 = 8;

pub struct World {
    pub fun: Fun,
    pub mode: Mode,
    pub game: Game,
    pub pogo_height: f64,
    /// Whether a rising pogo's drawn head stops at a ceiling (starts as `POGO_HEAD_BOX`).
    pub pogo_head_box: bool,
    pub held: u32,
    pub prev_held: u32,
    pub edge: u32,
    pub tick_count: u32,
    pub rng: Rng,
    pub t: f64,
    pub half_w: f64,
    pub half_h: f64,
    /// Ticks since the current attract level loaded, which level of the loop it is, and whether
    /// the camera holds still (reduced motion).
    pub attract_t: u32,
    pub attract_idx: u32,
    pub attract_still: bool,

    // Camera (world units). `p` prefix = previous tick, for interpolation.
    pub cam_x: f64,
    pub cam_y: f64,
    pub pcx: f64,
    pub pcy: f64,
    pub shake: f64,

    // Scene (level or overworld)
    pub level_id: u8,
    pub theme: Theme,
    pub map: TileMap,
    pub ents: Vec<Ent>,
    pub items: Vec<Item>,
    pub plats: Vec<Platform>,
    pub shots: Vec<Shot>,
    pub fx: Vec<Fx>,
    pub static_lights: Vec<StaticLight>,
    pub hazards: Vec<Hazard>,
    pub p: Player,
    pub keys_red: bool,
    pub keys_blue: bool,
    pub keys_green: bool,
    pub has_usb: bool,
    pub hacked: bool,
    pub won: bool,
    pub end_timer: Option<f64>,
    pub arena: Option<Arena>,
    /// Hidden rooms and how opaque each one's front is (1 hides it, a low value shows it).
    pub rooms: Vec<Room>,
    pub room_alpha: Vec<f64>,
    pub solid_count: usize,
    pub points: Vec<MapPoint>,
    /// Which overworld area each part of the map belongs to (map scene only).
    pub areas: Vec<levels::AreaRect>,
    /// The area of every overworld tile, row by row from the south, worked out when the map is entered so
    /// drawing it does not search the areas for each tile each frame.
    pub map_areas: Vec<u8>,
    pub near: Option<usize>,
    /// Seconds left before the map takes a press to open a level, so the press that closed a card
    /// does not open the level again the moment Ben is back.
    pub map_arm: f64,
    /// The level cleared for the first time, until the map is next entered and its flag is thrown.
    pub fresh_clear: Option<u8>,
    /// A flag in flight to its pole on the map: the level and seconds since it left Ben.
    pub flag_throw: Option<(u8, f64)>,
    /// The overworld's lights, worked out when the map is entered.
    pub map_lights: Vec<levels::MapLight>,
    /// The cleared-levels mask the lights were worked out for: they are rebuilt when it changes.
    pub map_lights_done: u32,
    /// The ambient light on the overworld now: it eases towards that of the area Ben is in.
    pub map_amb: [f32; 3],

    // Output buffers shared with the shell
    pub inst: InstanceBuffer,
    pub lights: LightPool,
    pub lights_pos: [f32; 64],
    pub lights_col: [f32; 48],
    pub events: EventQueue,
    pub spr: Vec<SpriteRect>,
    pub out: [f32; crate::render::out::LEN],
}

pub fn sign(x: f64) -> f64 {
    if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else {
        0.0
    }
}

fn sign_or1(x: f64) -> f64 {
    let s = sign(x);
    if s == 0.0 {
        1.0
    } else {
        s
    }
}

pub fn ground_ahead(map: &TileMap, e: &Ent) -> bool {
    let fx = if e.dir > 0.0 {
        e.b.x + e.b.w + 0.05
    } else {
        e.b.x - 0.05
    };
    let cx = fx.floor() as i32;
    let cy = (e.b.y - 0.1).floor() as i32;
    let t = map.get(cx, cy);
    let t2 = map.get(cx, e.b.y.floor() as i32);
    if t == SPIKE || is_liquid(t) || t2 == SPIKE {
        return false;
    }
    map.solid(cx, cy, true, e.b.y) || map.is_slope(t) || map.is_slope(t2)
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

impl World {
    pub fn new() -> Self {
        World {
            mode: Mode::None,
            game: Game::fresh(),
            pogo_height: 6.6,
            pogo_head_box: POGO_HEAD_BOX,
            held: 0,
            prev_held: 0,
            edge: 0,
            tick_count: 0,
            rng: Rng::new(0x5eed),
            t: 0.0,
            half_w: 10.0,
            half_h: 6.5,
            attract_t: 0,
            attract_idx: 0,
            attract_still: false,
            cam_x: 0.0,
            cam_y: 0.0,
            pcx: 0.0,
            pcy: 0.0,
            shake: 0.0,
            level_id: 0,
            theme: Theme::Crater,
            map: TileMap::new(1, 1, props()),
            ents: vec![],
            items: vec![],
            plats: vec![],
            shots: vec![],
            fx: vec![],
            static_lights: vec![],
            hazards: vec![],
            p: Player::level(0.0, 0.0),
            keys_red: false,
            keys_blue: false,
            keys_green: false,
            has_usb: false,
            hacked: false,
            won: false,
            end_timer: None,
            arena: None,
            rooms: vec![],
            room_alpha: vec![],
            solid_count: 0,
            points: vec![],
            areas: vec![],
            map_areas: vec![],
            near: None,
            map_arm: 0.0,
            fresh_clear: None,
            flag_throw: None,
            map_lights: vec![],
            map_lights_done: 0,
            map_amb: [1.0; 3],
            inst: InstanceBuffer::new(MAX_INSTANCES),
            lights: LightPool::new(256),
            lights_pos: [0.0; 64],
            lights_col: [0.0; 48],
            events: EventQueue::new(256),
            spr: vec![SpriteRect::default(); crate::sprites::SPRITE_NAMES.len()],
            out: [0.0; crate::render::out::LEN],
            fun: Fun::default(),
        }
    }

    /// Copies everything the simulation reads, with minimal output buffers, so search bots can
    /// branch a world cheaply. Rendering a fork is not supported.
    #[cfg(test)]
    pub fn fork(&self) -> World {
        World {
            mode: self.mode,
            game: self.game.clone(),
            pogo_height: self.pogo_height,
            pogo_head_box: self.pogo_head_box,
            held: self.held,
            prev_held: self.prev_held,
            edge: self.edge,
            tick_count: self.tick_count,
            rng: self.rng.clone(),
            t: self.t,
            half_w: self.half_w,
            half_h: self.half_h,
            attract_t: self.attract_t,
            attract_idx: self.attract_idx,
            attract_still: self.attract_still,
            cam_x: self.cam_x,
            cam_y: self.cam_y,
            pcx: self.pcx,
            pcy: self.pcy,
            shake: self.shake,
            level_id: self.level_id,
            theme: self.theme,
            map: self.map.clone(),
            ents: self.ents.clone(),
            items: self.items.clone(),
            plats: self.plats.clone(),
            shots: self.shots.clone(),
            fx: self.fx.clone(),
            static_lights: self.static_lights.clone(),
            hazards: self.hazards.clone(),
            p: self.p.clone(),
            keys_red: self.keys_red,
            keys_blue: self.keys_blue,
            keys_green: self.keys_green,
            has_usb: self.has_usb,
            hacked: self.hacked,
            won: self.won,
            end_timer: self.end_timer,
            arena: self.arena,
            rooms: self.rooms.clone(),
            room_alpha: self.room_alpha.clone(),
            solid_count: self.solid_count,
            points: self.points.clone(),
            areas: self.areas.clone(),
            map_areas: self.map_areas.clone(),
            near: self.near,
            map_arm: self.map_arm,
            fresh_clear: self.fresh_clear,
            flag_throw: self.flag_throw,
            map_lights: self.map_lights.clone(),
            map_lights_done: self.map_lights_done,
            map_amb: self.map_amb,
            inst: InstanceBuffer::new(1),
            lights: LightPool::new(1),
            lights_pos: [0.0; 64],
            lights_col: [0.0; 48],
            events: EventQueue::new(256),
            spr: Vec::new(),
            out: [0.0; crate::render::out::LEN],
            fun: self.fun,
        }
    }

    // ---------- Flow ----------

    pub fn game_new(&mut self) {
        self.game = Game::fresh();
        self.fresh_clear = None;
        self.flag_throw = None;
        self.hud();
    }

    pub fn hud(&mut self) {
        self.events.emit(ev::HUD, 0.0, 0.0, 0.0);
    }

    pub fn cap(&mut self, x: f64, y: f64, c: Cap) {
        self.events.emit(ev::CAPTION, x, y, f64::from(c as u16));
    }

    /// Shows a one-off joke toast, once per level visit.
    fn joke(&mut self, bit: u8, t: Toast) {
        if self.fun.told & bit == 0 {
            self.fun.told |= bit;
            self.toast(t);
        }
    }

    fn toast(&mut self, t: Toast) {
        self.events.emit(ev::TOAST, f64::from(t as u16), 0.0, 0.0);
    }

    /// Loads one level of the attract loop (Crater Fields, Crystal Caves, Mildred's Citadel, then
    /// round again). With `still` the camera holds at the start of the level instead of panning.
    pub fn load_attract(&mut self, idx: u32, still: bool) {
        self.attract_idx = idx % ATTRACT_LEVELS.len() as u32;
        self.load_level(ATTRACT_LEVELS[self.attract_idx as usize]);
        self.mode = Mode::Attract;
        self.p.hidden = true;
        self.attract_t = 0;
        self.attract_still = still;
        self.attract_camera();
    }

    /// How far left and right the attract camera pans: from just inside the left wall, up to
    /// `ATTRACT_SPAN` tiles, but never past the right wall.
    fn attract_range(&self) -> (f64, f64) {
        let w = f64::from(self.map.w);
        let x0 = self.half_w + 1.0;
        let x1 = (w - self.half_w - 1.0).min(x0 + ATTRACT_SPAN).max(x0 + 1.0);
        (x0, x1)
    }

    /// Ticks for the camera to pan across and back, which is how long one level is shown.
    pub fn attract_period(&self) -> u32 {
        let (x0, x1) = self.attract_range();
        ((2.0 * (x1 - x0) / ATTRACT_SPEED) * 60.0).ceil() as u32
    }

    /// Places the camera from the tick count alone, so the same ticks always give the same view.
    /// The bottom edge sits on row 0 and never shows below the bottom tile row.
    fn attract_camera(&mut self) {
        let (x0, x1) = self.attract_range();
        let dist = x1 - x0;
        let period = f64::from(self.attract_period().max(1));
        let ph = if self.attract_still {
            0.0
        } else {
            f64::from(self.attract_t % self.attract_period().max(1)) / period
        };
        self.cam_x = x0 + dist * if ph < 0.5 { ph * 2.0 } else { 2.0 - ph * 2.0 };
        let (w, h) = (f64::from(self.map.w), f64::from(self.map.h));
        let (hw, hh) = (self.half_w, self.half_h);
        self.cam_x = if hw * 2.0 < w {
            self.cam_x.clamp(hw, w - hw)
        } else {
            w / 2.0
        };
        self.cam_y = if hh * 2.0 < h { hh } else { h / 2.0 };
        self.pcx = self.cam_x;
        self.pcy = self.cam_y;
    }

    /// Where Ben stands when he is back on the map from the level at node `(x, y)`: at the node's doorstep,
    /// close enough that its prompt is up and one press opens the level again. That is the tile in front
    /// of it (south, since y is up) when that is clear, else the node's own tile, which is always walkable.
    fn doorstep(&self, x: f64, y: f64) -> (f64, f64) {
        [(x + 0.2, y - 0.4), (x + 0.2, y + 0.1)]
            .into_iter()
            .find(|&p| map_body_free(&self.map, p))
            .unwrap_or((x + 0.2, y + 0.1))
    }

    pub fn enter_level(&mut self, id: u8) {
        let id = if id < levels::TABLE_LEN { id } else { 0 };
        if self.mode == Mode::Map {
            if let Some(pt) = self
                .points
                .iter()
                .find(|p| matches!(p.kind, PtKind::Level | PtKind::Saucer) && p.level == id)
            {
                self.game.map_pos = Some(self.doorstep(pt.x, pt.y));
            }
        }
        self.load_level(id);
        self.mode = Mode::Level;
        self.hud();
        self.events.emit(ev::LEVEL_START, f64::from(id), 0.0, 0.0);
    }

    pub fn load_level(&mut self, id: u8) {
        // An id the table does not know builds the first level, so it is also that level's id.
        let id = if id < levels::TABLE_LEN { id } else { 0 };
        let d = levels::build_level(id);
        self.level_id = id;
        self.theme = d.theme;
        self.map = d.map;
        self.t = 0.0;
        self.shots.clear();
        self.fx.clear();
        self.keys_red = false;
        self.keys_blue = false;
        self.keys_green = false;
        self.has_usb = false;
        self.hacked = false;
        self.won = false;
        self.fun = Fun::default();
        self.end_timer = None;
        self.shake = 0.0;
        self.near = None;
        self.points.clear();
        self.items = d.items;
        self.plats = d.plats;
        self.arena = d.arena;
        self.room_alpha = vec![1.0; d.rooms.len()];
        self.rooms = d.rooms;
        self.ents = d.spawns.iter().map(|s| self.init_ent(s)).collect();
        self.p = Player::level(d.start.0, d.start.1);
        self.cam_x = d.start.0 + 6.0;
        self.cam_y = d.start.1 + 3.0;
        self.pcx = self.cam_x;
        self.pcy = self.cam_y;
        let (w, h) = (self.map.w, self.map.h);
        let bio = crate::render::theme(self.theme);
        self.static_lights.clear();
        for y in 0..h {
            for x in 0..w {
                let t = self.map.get(x, y);
                if t == CRYS {
                    self.static_lights.push(StaticLight {
                        x: f64::from(x) + 0.5,
                        y: f64::from(y) + 0.7,
                        c: bio.lc,
                    });
                }
                if t == EXIT && self.map.get(x, y + 1) != EXIT {
                    self.static_lights.push(StaticLight {
                        x: f64::from(x) + 0.5,
                        y: f64::from(y) + 0.4,
                        c: [0.35, 0.9, 0.4],
                    });
                }
            }
        }
        self.solid_count = self.map.data.iter().filter(|&&t| t != 0).count();
        self.hazards.clear();
        for y in 0..h {
            let mut run: i32 = -1;
            for x in 0..=w {
                let t = if x < w { self.map.get(x, y) } else { 0 };
                let top = t == SPIKE || (is_liquid(t) && !is_liquid(self.map.get(x, y + 1)));
                if top && run < 0 {
                    run = x;
                }
                if !top && run >= 0 {
                    self.hazards.push(Hazard {
                        x: f64::from(run + x) / 2.0,
                        y: f64::from(y) + 0.7,
                        w: f64::from(x - run),
                    });
                    run = -1;
                }
            }
        }
    }

    pub(crate) fn init_ent(&self, s: &Spawn) -> Ent {
        let d = info(s.kind);
        let mut e = Ent {
            kind: s.kind,
            b: Body::new(s.x - d.w / 2.0 + 0.5, s.y, d.w, d.h),
            dir: s.dir,
            stun: 0.0,
            t: hashf(s.x as i32, 7) * 10.0,
            cd: 0.0,
            fire_t: 1.5,
            state: St::Idle,
            st: 0.0,
            ax: 0.0,
            ay: 0.0,
            rot: 0.0,
            alpha: 1.0,
            hp: 0,
            puff: false,
            touch: false,
            dead: false,
            stray: 0.0,
            ride: s.ride,
        };
        e.ax = e.b.x;
        e.ay = e.b.y;
        if s.kind == Kind::Bat {
            let (w, h) = (self.map.w, self.map.h);
            let _ = w;
            let mut y = s.y.floor() as i32;
            let cx = s.x.floor() as i32;
            while y < h - 1 && !self.map.is_solid_tile(self.map.get(cx, y)) {
                y += 1;
            }
            e.b.y = f64::from(y) - e.b.h;
            e.ay = e.b.y;
            e.b.py = e.b.y;
        }
        if s.kind == Kind::Boss {
            e.hp = 3;
            e.state = St::Wait;
            e.b.x = s.x;
            e.b.px = s.x;
            e.ax = s.x;
        }
        e
    }

    pub fn enter_map(&mut self) {
        let m = levels::build_overworld();
        self.map = m.map;
        self.points = m.points;
        self.areas = m.areas;
        let (mw, mh) = (self.map.w, self.map.h);
        let mut map_areas = Vec::with_capacity((mw * mh) as usize);
        for y in 0..mh {
            for x in 0..mw {
                map_areas.push(levels::area_at(&self.areas, x, y));
            }
        }
        self.map_areas = map_areas;
        self.mode = Mode::Map;
        self.t = 0.0;
        self.shake = 0.0;
        self.fx.clear();
        self.shots.clear();
        self.ents.clear();
        self.items.clear();
        self.plats.clear();
        self.near = None;
        self.map_arm = MAP_ARM_SECS;
        self.flag_throw = self
            .fresh_clear
            .take()
            .filter(|&l| self.game.is_done(l))
            .map(|l| (l, 0.0));
        // A saved position may belong to an older map: if Ben would start inside a river, a tree or
        // a rock, put him back at the saucer instead.
        let at = self
            .game
            .map_pos
            .filter(|&p| map_body_free(&self.map, p))
            .unwrap_or(m.start);
        let mut b = Body::new(at.0, at.1, 0.6, 0.6);
        b.px = at.0;
        b.py = at.1;
        self.p = Player::level(at.0, at.1);
        self.p.b = b;
        self.cam_x = at.0;
        self.cam_y = at.1;
        self.pcx = at.0;
        self.pcy = at.1;
        self.rebuild_map_lights();
        self.map_amb = self.map_ambient_here();
        self.hud();
        self.events.emit(ev::MAP_PROMPT, 0.0, 0.0, 0.0);
    }

    /// The ambient light of the overworld area Ben stands in.
    fn map_ambient_here(&self) -> [f32; 3] {
        let (x, y) = (
            (self.p.b.x + 0.3).floor() as i32,
            (self.p.b.y + 0.3).floor() as i32,
        );
        levels::map_ambient(levels::area_at(&self.areas, x, y))
    }

    // ---------- Tick ----------

    pub fn step(&mut self, held: u32) {
        self.edge = held & !self.prev_held;
        self.prev_held = held;
        self.held = held;
        if self.mode == Mode::None {
            return;
        }
        let dt = STEP;
        self.tick_count = self.tick_count.wrapping_add(1);
        self.t += dt;
        self.p.b.px = self.p.b.x;
        self.p.b.py = self.p.b.y;
        self.pcx = self.cam_x;
        self.pcy = self.cam_y;
        for f in &mut self.fx {
            f.t += dt;
        }
        self.fx.retain(|f| f.t < f.life);
        match self.mode {
            Mode::Map => {
                self.tick_map(dt);
                self.follow(dt, 0.0, 0.0, 3.5);
            }
            Mode::Attract => {
                self.attract_t = self.attract_t.wrapping_add(1);
                self.attract_camera();
                self.tick_ents(dt);
            }
            Mode::Level => {
                for pl in &mut self.plats {
                    pl.tick(dt);
                }
                self.tick_ben(dt);
                self.tick_rooms(dt);
                self.tick_ents(dt);
                self.tick_shots(dt);
                if let Some(t) = self.end_timer {
                    if t - dt <= 0.0 {
                        self.end_timer = None;
                        self.events.emit(ev::ENDING, 0.0, 0.0, 0.0);
                    } else {
                        self.end_timer = Some(t - dt);
                    }
                }
                let p = &self.p;
                match self.theme.cam() {
                    CamMode::Side => {
                        let up = if p.look_down > 0.2 { -3.6 } else { 1.6 };
                        self.follow(dt, p.face * 1.5, up, 3.5);
                    }
                    CamMode::Tower => {
                        // Tall levels: no look-ahead, a low aim point and a quick vertical chase so
                        // a long fall or a lift ride never leaves Ben off screen.
                        let up = if p.look_down > 0.2 {
                            -3.6
                        } else if p.look_up > 0.2 {
                            3.6
                        } else {
                            0.8
                        };
                        self.follow(dt, 0.0, up, 8.0);
                    }
                }
            }
            Mode::None => {}
        }
        // Shake fades in every ticking mode: a hit that lands just before the level ends, or a
        // bump in the attract loop, must not leave the camera vibrating on the map.
        if self.shake > 0.0 {
            self.shake = (self.shake - dt * 2.0).max(0.0);
        }
    }

    fn follow(&mut self, dt: f64, look: f64, up: f64, rate_y: f64) {
        let (hw, hh) = (self.half_w, self.half_h);
        if self.p.dead == 0.0 {
            let tx = self.p.b.x + self.p.b.w / 2.0 + look;
            let ty = self.p.b.y + up;
            self.cam_x += (tx - self.cam_x) * (5.0 * dt).min(1.0);
            self.cam_y += (ty - self.cam_y) * (rate_y * dt).min(1.0);
        }
        let (w, h) = (f64::from(self.map.w), f64::from(self.map.h));
        self.cam_x = if hw * 2.0 < w {
            self.cam_x.clamp(hw, w - hw)
        } else {
            w / 2.0
        };
        self.cam_y = if hh * 2.0 < h {
            self.cam_y.clamp(hh, h - hh)
        } else {
            h / 2.0
        };
    }

    // ---------- Overworld ----------

    /// Works the overworld's lights out again: which teleporters are on and which levels show depend on
    /// what has been cleared.
    fn rebuild_map_lights(&mut self) {
        self.map_lights = levels::map_lights(&self.map, &self.points, |req| self.met(req));
        self.map_lights_done = self.game.done;
    }

    fn tick_map(&mut self, dt: f64) {
        self.map_arm = (self.map_arm - dt).max(0.0);
        if self.map_lights_done != self.game.done {
            self.rebuild_map_lights();
        }
        if let Some((l, t)) = self.flag_throw {
            self.flag_throw = (t + dt < FLAG_THROW_SECS).then_some((l, t + dt));
        }
        // The light eases from one area's to the next over about a second, so crossing a river is not a jump.
        let target = self.map_ambient_here();
        let k = (1.0 - (-dt / 0.4).exp()) as f32;
        for (a, t) in self.map_amb.iter_mut().zip(target) {
            *a += (t - *a) * k;
        }
        let h = self.held;
        let vx = f64::from(u8::from(h & RIGHT != 0)) - f64::from(u8::from(h & LEFT != 0));
        let vy = f64::from(u8::from(h & UP != 0)) - f64::from(u8::from(h & DOWN != 0));
        let l = vx.hypot(vy);
        let l = if l == 0.0 { 1.0 } else { l };
        self.p.b.vx = vx / l * 5.0;
        self.p.b.vy = vy / l * 5.0;
        if vx != 0.0 {
            self.p.face = vx;
        }
        let (dx, dy) = (self.p.b.vx * dt, self.p.b.vy * dt);
        self.p.b.move_x(&self.map, dx, false);
        self.p.b.move_y(&self.map, dy);
        if vx != 0.0 || vy != 0.0 {
            self.p.anim += dt * 6.0;
        }
        let mut near = None;
        for (i, pt) in self.points.iter().enumerate() {
            if pt.hidden && !self.met(pt.req) {
                continue;
            }
            let d = (self.p.b.x + 0.3 - (pt.x + 0.5)).hypot(self.p.b.y + 0.3 - (pt.y + 0.5));
            if d < if pt.big { 1.6 } else { 0.95 } {
                near = Some(i);
            }
        }
        if near != self.near {
            self.near = near;
            match near {
                None => self.events.emit(ev::MAP_PROMPT, 0.0, 0.0, 0.0),
                Some(i) => {
                    let pt = self.points[i];
                    match pt.kind {
                        PtKind::Level if !self.met(pt.req) => {
                            // Locked: name the first level still to clear, and carry this level's id.
                            self.events.emit(
                                ev::MAP_PROMPT,
                                4.0,
                                f64::from(self.first_unmet(pt.req)),
                                f64::from(pt.level),
                            );
                        }
                        PtKind::Level => {
                            let done = self.game.is_done(pt.level);
                            self.events.emit(
                                ev::MAP_PROMPT,
                                1.0,
                                f64::from(pt.level),
                                f64::from(u8::from(done)),
                            );
                        }
                        PtKind::Tele if !self.met(pt.req) && pt.req & SECRET_FOUND as u16 != 0 => {
                            // The island pad: nothing on the map tells Ben how to wake it.
                            self.events.emit(ev::MAP_PROMPT, 5.0, 0.0, 0.0);
                        }
                        PtKind::Tele => {
                            let done = self.met(pt.req);
                            self.events.emit(
                                ev::MAP_PROMPT,
                                2.0,
                                f64::from(self.first_unmet(pt.req)),
                                f64::from(u8::from(done)),
                            );
                        }
                        PtKind::Saucer => self.events.emit(ev::MAP_PROMPT, 3.0, 0.0, 0.0),
                        PtKind::Sign => {
                            self.events
                                .emit(ev::MAP_PROMPT, 6.0, f64::from(pt.level), 0.0)
                        }
                    }
                }
            }
        }
        if let Some(i) = near {
            if self.map_arm <= 0.0 && self.edge & (JUMP | FIRE | CONFIRM) != 0 {
                self.activate(i);
            }
        }
    }

    /// True when every level in the `req` mask has been cleared (0 means no requirement).
    pub fn met(&self, req: u16) -> bool {
        self.game.done & u32::from(req) == u32::from(req)
    }

    /// Lowest-numbered level in `req` that is not yet cleared, or 0 when all are.
    fn first_unmet(&self, req: u16) -> u8 {
        let missing = u32::from(req) & !self.game.done;
        if missing == 0 {
            0
        } else {
            missing.trailing_zeros() as u8
        }
    }

    fn activate(&mut self, i: usize) {
        let pt = self.points[i];
        match pt.kind {
            PtKind::Level if self.met(pt.req) => self.enter_level(pt.level),
            PtKind::Saucer => self.enter_level(pt.level),
            PtKind::Tele if self.met(pt.req) => {
                let to = self.points[pt.to];
                let (px, py) = (self.p.b.x, self.p.b.y);
                self.cap(px, py + 1.0, Cap::Vworp);
                // Land on the pad's own tile: a 0.6-tile body there covers one tile of ground, which
                // is always walkable, whatever the scenery is like around the pad.
                self.p.b.x = to.x + 0.2;
                self.p.b.y = to.y + 0.1;
                self.p.b.px = self.p.b.x;
                self.p.b.py = self.p.b.y;
                self.cam_x = self.p.b.x;
                self.cam_y = self.p.b.y;
                self.pcx = self.cam_x;
                self.pcy = self.cam_y;
                self.near = None;
            }
            _ => {}
        }
    }

    // ---------- Player ----------

    fn tick_ben(&mut self, dt: f64) {
        if self.p.dead > 0.0 {
            let p = &mut self.p;
            p.dead += dt;
            p.b.vy -= 30.0 * dt;
            p.b.x += p.b.vx * dt;
            p.b.y += p.b.vy * dt;
            p.rot += dt * 8.0;
            if p.dead > 1.4 && !p.dead_sent {
                p.dead_sent = true;
                self.on_death();
            }
            return;
        }
        if self.won {
            return;
        }
        if self.p.inv > 0.0 {
            self.p.inv -= dt;
        }
        if self.p.shoot_t > 0.0 {
            self.p.shoot_t -= dt;
        }
        if self.p.wall_t > 0.0 {
            self.p.wall_t -= dt;
        }
        if self.p.b.on_ground || self.p.climb {
            self.p.kick_side = 0.0;
        }
        if let Some(i) = self.p.b.on_plat {
            let (dx, dy) = (self.plats[i].dx, self.plats[i].dy);
            self.p.b.x += dx;
            self.p.b.y += dy;
        }
        // A mantle owns Ben's movement until it ends: no running, jumping, firing or physics.
        if self.p.mantle.is_some() {
            self.tick_mantling(dt);
        } else {
            self.tick_ben_move(dt);
        }
        self.tick_ben_contacts();
    }

    /// Ben's own movement for one tick: input, jumps, kicks, climbing, shots and physics.
    fn tick_ben_move(&mut self, dt: f64) {
        let (h, e) = (self.held, self.edge);
        if e & POGO != 0 && !self.p.climb {
            self.p.pogo = !self.p.pogo;
            if !self.p.pogo {
                self.fun.pogos = 0;
            }
        }
        if h == 0 && self.p.b.on_ground && self.p.dead == 0.0 {
            self.fun.idle += dt;
            if self.fun.idle > 20.0 {
                self.joke(JOKE_IDLE, Toast::JokeIdle);
            }
        } else {
            self.fun.idle = 0.0;
        }
        let ax = f64::from(u8::from(h & RIGHT != 0)) - f64::from(u8::from(h & LEFT != 0));
        let jump_held = h & JUMP != 0;
        let was_climbing = self.p.climb;
        self.tick_climb(h, e, ax);
        {
            let p = &mut self.p;
            if ax != 0.0 {
                p.face = ax;
            }
            let idle = p.b.on_ground && !p.pogo && !p.climb && ax == 0.0;
            p.look_down = if h & DOWN != 0 && h & UP == 0 && idle {
                p.look_down + dt
            } else {
                0.0
            };
            p.look_up = if h & UP != 0 && h & DOWN == 0 && idle {
                p.look_up + dt
            } else {
                0.0
            };
            let b = &mut p.b;
            if b.on_ground && !p.pogo {
                if ax != 0.0 {
                    b.vx += ax * if sign(b.vx) == -ax { 90.0 } else { 55.0 } * dt;
                } else {
                    let f = 60.0 * dt;
                    b.vx = if b.vx.abs() <= f {
                        0.0
                    } else {
                        b.vx - sign(b.vx) * f
                    };
                }
            } else {
                b.vx += ax * 32.0 * dt;
                if ax == 0.0 {
                    b.vx *= 1.0 - 1.5 * dt;
                }
            }
            b.vx = b.vx.clamp(-7.0, 7.0);
        }
        if self.p.b.on_ground {
            if self.p.pogo {
                let hgt = self.pogo_height;
                self.p.b.vy = if jump_held {
                    (2.0 * GRAVITY * hgt).sqrt()
                } else {
                    14.0
                };
                self.p.b.on_ground = false;
                self.p.squash = 0.12;
                let (x, y) = (self.p.b.x + 0.35, self.p.b.y);
                self.cap(x, y, Cap::Boing);
                self.fun.pogos += 1;
                if self.fun.pogos >= 30 {
                    self.joke(JOKE_POGO, Toast::JokePogo);
                }
            } else if e & JUMP != 0 {
                let (x, y) = (self.p.b.x, self.p.b.y);
                self.cap(x, y, Cap::Jump);
                self.p.b.vy = 20.5;
                self.p.cut = true;
                self.p.b.on_ground = false;
            }
        } else if e & JUMP != 0
            && !was_climbing
            && !self.p.climb
            && !self.p.pogo
            && self.p.wall_t <= 0.0
        {
            // In the air against a wall, Jump kicks off it: up and away.
            if let Some(side) = self.wall_side().filter(|&s| s != self.p.kick_side) {
                let (x, y) = (self.p.b.x + 0.35, self.p.b.y + 0.7);
                self.cap(x, y, Cap::Kick);
                let p = &mut self.p;
                p.b.vy = WALL_KICK_UP;
                p.b.vx = -side * WALL_KICK_AWAY;
                p.face = -side;
                p.cut = false;
                p.wall_t = WALL_KICK_COOLDOWN;
                p.kick_side = side;
                self.fun.kicks += 1;
                if self.fun.kicks >= 12 {
                    self.joke(JOKE_KICK, Toast::JokeKick);
                }
            }
        }
        {
            let p = &mut self.p;
            if p.cut && !jump_held && p.b.vy > 0.0 {
                p.b.vy *= 0.45;
                p.cut = false;
            }
            if p.b.vy <= 0.0 {
                p.cut = false;
            }
            if p.squash > 0.0 {
                p.squash -= dt;
            }
            p.b.vy = (p.b.vy - GRAVITY * dt).max(-22.0);
        }
        if self.p.climb {
            let dir = f64::from(u8::from(h & UP != 0)) - f64::from(u8::from(h & DOWN != 0));
            if self.p.wall {
                // A vine can be crossed sideways as well as climbed, more slowly than a ladder.
                self.p.b.vx = VINE_SIDE_SPEED * ax;
                self.p.b.vy = VINE_SPEED * dir;
            } else {
                self.p.b.vx = 0.0;
                self.p.b.vy = CLIMB_SPEED * dir;
            }
        }
        if e & FIRE != 0 {
            self.fire();
        }
        let rising = self.p.b.vy;
        // Only a rising pogo has the taller head: walking, falling and standing use the body alone.
        self.p.b.head = if self.pogo_head_box && self.p.pogo && rising > 0.0 {
            POGO_DRAWN_HEIGHT - self.p.b.h
        } else {
            0.0
        };
        self.p.b.phys(&self.map, &self.plats, dt);
        self.p.b.head = 0.0;
        if self.p.b.bonk && self.p.pogo {
            self.p.b.vy = 0.0;
        }
        self.tick_head_bump(rising);
        self.p.anim += if self.p.climb {
            self.p.b.vy.abs() + self.p.b.vx.abs()
        } else {
            self.p.b.vx.abs()
        } * dt;
        self.tick_mantle(ax);
    }

    /// Raises the "clunk" when Ben's head meets a ceiling while he rises, once per rise. The hit zeroes
    /// his upward speed, so a held jump or a fall under the same ceiling does not repeat it, and
    /// climbing (which sets his speed every tick) stays quiet. `bumped` also holds a pogo that bounces
    /// again at once under a low ceiling to a single clunk until he stops bumping. `rising` is his
    /// upward speed before the move.
    fn tick_head_bump(&mut self, rising: f64) {
        if self.p.b.vy <= 0.0 && !self.p.b.bonk {
            self.p.bumped = false;
        }
        if rising <= 0.0 || self.p.b.on_ground {
            self.p.rise_y = self.p.b.y;
        }
        // A rise that gains no height since it began (a pogo under a ceiling that just fits it,
        // bouncing again at once from the floor) is not a bump, so a tight corridor does not rattle.
        if self.p.bumped
            || self.p.climb
            || rising <= HEAD_BUMP_MIN_SPEED
            || !self.p.b.bonk
            || self.p.b.y - self.p.rise_y < HEAD_BUMP_MIN_RISE
        {
            return;
        }
        let b = &self.p.b;
        let (x, y) = (b.x + b.w / 2.0, b.y + b.h);
        self.p.bumped = true;
        self.cap(x, y, Cap::Clunk);
    }

    /// Hazards, doors, items, enemy contact and the exit, at wherever Ben ended up this tick.
    fn tick_ben_contacts(&mut self) {
        // Hazards, doors, exit.
        let (px, py, pw, ph) = (self.p.b.x, self.p.b.y, self.p.b.w, self.p.b.h);
        let mut die = py < -1.5;
        let mut why = (death::FELL, 0u16);
        for cx in (px.floor() as i32)..=((px + pw).floor() as i32) {
            for cy in (py.floor() as i32)..=((py + ph).floor() as i32) {
                let t = self.map.get(cx, cy);
                if t == SPIKE && py < f64::from(cy) + 0.55 {
                    if !die {
                        why = (death::SPIKES, u16::from(t));
                    }
                    die = true;
                }
                if is_liquid(t) {
                    let lim = if is_liquid(self.map.get(cx, cy + 1)) {
                        1.0
                    } else {
                        0.55
                    };
                    if py < f64::from(cy) + lim {
                        if !die {
                            why = (
                                if t == CHOC {
                                    death::CHOCOLATE
                                } else {
                                    death::MOLTEN_METAL
                                },
                                u16::from(t),
                            );
                        }
                        die = true;
                    }
                }
                if t == EXIT && !self.won {
                    self.won = true;
                    self.p.mantle = None;
                    if !self.game.is_done(self.level_id) {
                        self.fresh_clear = Some(self.level_id);
                    }
                    self.game.set_done(self.level_id);
                    self.cap(px, py + 2.0, Cap::TaDa);
                    self.hud();
                    self.events
                        .emit(ev::LEVEL_COMPLETE, f64::from(self.level_id), 0.0, 0.0);
                    return;
                }
            }
        }
        for (tt, colour) in [(DOOR_R, 0u8), (DOOR_B, 1), (DOOR_G, 2)] {
            let has = match colour {
                0 => self.keys_red,
                1 => self.keys_blue,
                _ => self.keys_green,
            };
            if !has {
                continue;
            }
            let cx = if self.p.face > 0.0 {
                (px + pw + 0.1).floor() as i32
            } else {
                (px - 0.1).floor() as i32
            };
            if self.map.get(cx, (py + 0.2).floor() as i32) == tt {
                // In a building the gap shows the interior wall, not the sky behind it.
                let open = if self.theme == Theme::Building {
                    WALLBG
                } else {
                    0
                };
                for v in self.map.data.iter_mut() {
                    if *v == tt {
                        *v = open;
                    }
                }
                match colour {
                    0 => self.keys_red = false,
                    1 => self.keys_blue = false,
                    _ => self.keys_green = false,
                }
                self.cap(f64::from(cx) + 0.5, py + 2.0, Cap::Clunk);
                self.hud();
                self.toast(match colour {
                    0 => Toast::RedDoor,
                    1 => Toast::BlueDoor,
                    _ => Toast::GreenDoor,
                });
            }
        }
        if die {
            self.kill_by(why.0, why.1);
            return;
        }

        // Items
        for i in 0..self.items.len() {
            let it = self.items[i];
            if it.taken || !self.p.b.overlaps(it.x - 0.4, it.y - 0.4, 0.8, 0.8) {
                continue;
            }
            self.items[i].taken = true;
            match it.kind {
                ItemKind::Cheezie | ItemKind::Choc | ItemKind::Cookie => {
                    self.game.score += match it.kind {
                        ItemKind::Cheezie => 1,
                        ItemKind::Choc => 2,
                        _ => 5,
                    };
                    self.cap(it.x, it.y + 0.6, Cap::Crunch);
                    while self.game.score >= self.game.next_life {
                        self.game.lives += 1;
                        self.game.next_life += 100;
                        self.cap(px, py + 2.2, Cap::ExtraLife);
                        self.toast(Toast::ExtraLife);
                    }
                }
                ItemKind::Soda => {
                    self.game.ammo += 5;
                    self.cap(it.x, it.y + 0.6, Cap::Fsssht);
                }
                ItemKind::KeyRed => {
                    self.keys_red = true;
                    self.cap(it.x, it.y + 0.6, Cap::RedGumdrop);
                }
                ItemKind::KeyBlue => {
                    self.keys_blue = true;
                    self.cap(it.x, it.y + 0.6, Cap::BlueGumdrop);
                }
                ItemKind::KeyGreen => {
                    self.keys_green = true;
                    self.cap(it.x, it.y + 0.6, Cap::GreenGumdrop);
                }
                ItemKind::Usb => {
                    self.has_usb = true;
                    self.cap(it.x, it.y + 0.6, Cap::GoldUsb);
                    self.toast(Toast::UsbFound);
                }
            }
            self.hud();
        }

        // Entity contact
        for i in 0..self.ents.len() {
            let en = self.ents[i].clone();
            let b = &en.b;
            if en.dead || !self.p.b.overlaps(b.x, b.y, b.w, b.h) {
                self.ents[i].touch = false;
                continue;
            }
            let first = !en.touch;
            self.ents[i].touch = true;
            let d = info(en.kind);
            match en.kind {
                Kind::Switch => {
                    if first {
                        let on = self.map.toggle_switch(0);
                        self.cap(b.x + 0.4, b.y + 1.4, Cap::ClickClack);
                        self.toast(if on {
                            Toast::BridgeOn
                        } else {
                            Toast::BridgeOff
                        });
                    }
                    continue;
                }
                Kind::Terminal => {
                    if first {
                        if self.has_usb && !self.hacked {
                            self.hacked = true;
                            self.has_usb = false;
                            self.hud();
                            self.cap(b.x + 0.5, b.y + 2.0, Cap::BeepBoop);
                            self.end_timer = Some(1.4);
                        } else if !self.hacked {
                            self.cap(b.x + 0.5, b.y + 2.0, Cap::NeedsDrive);
                        }
                    }
                    continue;
                }
                Kind::Cameo => {
                    if first {
                        self.toast(if en.dir > 0.0 {
                            Toast::CameoMortimer
                        } else {
                            Toast::CameoBilly
                        });
                    }
                    continue;
                }
                Kind::Glyph => {
                    if first {
                        let toast = match en.dir as i32 {
                            1 => Toast::GlyphVisitors,
                            4 => Toast::SaucerHatch,
                            5 => Toast::SaucerGalley,
                            6 => Toast::SaucerBunks,
                            7 => Toast::SaucerChart,
                            8 => Toast::SaucerShelf,
                            9 => Toast::SaucerBridge,
                            2 => Toast::GlyphFizz,
                            3 => Toast::GlyphDeep,
                            _ => Toast::GlyphGrowth,
                        };
                        self.toast(toast);
                    }
                    continue;
                }
                // A raised press is harmless; only one that has come down towards the floor crushes.
                Kind::Press if en.b.y >= en.ay - PRESS_DANGER_DROP => continue,
                _ => {}
            }
            if d.prop {
                continue;
            }
            let stomp = self.p.b.vy < 0.0 && self.p.b.py >= b.y + b.h * 0.55;
            if en.kind == Kind::Marsh {
                if stomp {
                    self.p.b.vy = if self.p.pogo { 26.0 } else { 18.0 };
                    let (x, y) = (self.p.b.x, self.p.b.y);
                    self.cap(x, y, Cap::Sproing);
                    self.fun.marsh += 1;
                    if self.fun.marsh >= 8 {
                        self.joke(JOKE_MARSH, Toast::JokeMarsh);
                    }
                } else if first {
                    let diff = self.p.b.x - b.x;
                    self.p.b.vx = sign(if diff == 0.0 { 1.0 } else { diff }) * 12.0;
                    self.p.b.vy = self.p.b.vy.max(7.0);
                    self.p.b.on_ground = false;
                    self.p.mantle = None;
                    self.cap(b.x, b.y + 1.2, Cap::Bwomp);
                }
                continue;
            }
            if en.stun > 0.0 || d.harmless {
                if stomp {
                    self.p.b.vy = if self.p.pogo { 20.0 } else { 12.0 };
                }
                continue;
            }
            if en.kind == Kind::Boss {
                if stomp && en.state == St::Hot {
                    self.hit_boss(i);
                    self.p.b.vy = 22.0;
                    continue;
                }
            } else if stomp && d.stun {
                self.ents[i].stun = 3.0;
                self.p.b.vy = if self.p.pogo { 20.0 } else { 12.0 };
                self.cap(b.x + b.w / 2.0, b.y + b.h + 0.4, Cap::Bonk);
                continue;
            }
            if en.kind == Kind::Boss && en.state == St::Down {
                continue;
            }
            if self.p.inv <= 0.0 {
                self.kill_by(death::ENEMY, en.kind as u16);
                return;
            }
        }
        for i in 0..self.ents.len() {
            let en = &self.ents[i];
            if en.kind == Kind::Pod && en.puff && en.stun <= 0.0 {
                let (x, y, w) = (en.b.x, en.b.y, en.b.w);
                if self.p.b.overlaps(x - 2.2, y, 2.1, 1.2)
                    || self.p.b.overlaps(x + w + 0.1, y, 2.1, 1.2)
                {
                    self.kill_by(death::PUFF, Kind::Pod as u16);
                    return;
                }
            }
        }
    }

    /// Fades each hidden room's front wall away while Ben is inside it and back once he leaves.
    fn tick_rooms(&mut self, dt: f64) {
        let b = &self.p.b;
        let mut found = false;
        for (r, a) in self.rooms.iter().zip(self.room_alpha.iter_mut()) {
            let inside = r.overlaps(b.x, b.y, b.w, b.h);
            found |= inside && r.secret;
            let target = if inside { ROOM_SEEN_ALPHA } else { 1.0 };
            *a += (target - *a) * (6.0 * dt).min(1.0);
        }
        if found && self.game.done & SECRET_FOUND == 0 {
            self.game.done |= SECRET_FOUND;
            let (x, y) = (self.p.b.x, self.p.b.y);
            self.cap(x, y + 1.5, Cap::Vworp);
            self.toast(Toast::SecretFound);
        }
    }

    /// Index of the hidden room covering tile (x, y), if any.
    pub fn room_at(&self, x: i32, y: i32) -> Option<usize> {
        self.rooms.iter().position(|r| r.contains(x, y))
    }

    /// Which side a solid wall touches Ben on at chest height: -1 on his left, 1 on his right.
    pub(crate) fn wall_side(&self) -> Option<f64> {
        let b = &self.p.b;
        let row = (b.y + 0.7).floor() as i32;
        let solid = |x: f64| self.map.is_solid_tile(self.map.get(x.floor() as i32, row));
        if solid(b.x - 0.08) {
            Some(-1.0)
        } else if solid(b.x + b.w + 0.08) {
            Some(1.0)
        } else {
            None
        }
    }

    /// Whether a box of Ben's size at (x, y) overlaps no solid tile.
    fn body_clear(&self, x: f64, y: f64) -> bool {
        self.region_clear(x, x, y, y)
    }

    /// Whether everything Ben's body covers while its corner moves anywhere in the box from
    /// (x0, y0) to (x1, y1) is free of solid tiles.
    fn region_clear(&self, x0: f64, x1: f64, y0: f64, y1: f64) -> bool {
        let b = &self.p.b;
        (x0.floor() as i32..=(x1 + b.w - 1e-4).floor() as i32).all(|cx| {
            (y0.floor() as i32..=(y1 + b.h - 1e-4).floor() as i32)
                .all(|cy| !self.map.solid(cx, cy, false, 0.0))
        })
    }

    /// Starts a pull-up onto a ledge Ben nearly made: in the air, pressing towards a wall whose top
    /// edge is just above his feet, with room to stand on it and to haul up the wall and over the
    /// lip, he grabs on. `tick_mantling` then carries him to the top over `MANTLE_TIME`.
    fn tick_mantle(&mut self, ax: f64) {
        let p = &self.p;
        if ax == 0.0 || p.b.on_ground || p.climb || p.pogo || p.dead > 0.0 || !p.b.hit_x {
            return;
        }
        let b = &p.b;
        let wx = if ax > 0.0 {
            (b.x + b.w + 0.1).floor() as i32
        } else {
            (b.x - 0.1).floor() as i32
        };
        // The wall's top edge: the highest solid tile in the column whose feet-to-top gap is in
        // reach and which has two free tiles above it.
        for r in (b.y.floor() as i32 - 1)..=((b.y + MANTLE_REACH).floor() as i32) {
            let top = f64::from(r + 1);
            let solid = |x: i32, y: i32| self.map.is_solid_tile(self.map.get(x, y));
            if !solid(wx, r) || solid(wx, r + 1) || solid(wx, r + 2) {
                continue;
            }
            let rise = top - b.y;
            if !(0.1..=MANTLE_REACH).contains(&rise) {
                continue;
            }
            let nx = if ax > 0.0 {
                f64::from(wx) + 0.1
            } else {
                f64::from(wx) + 1.0 - b.w - 0.1
            };
            // The whole path must be clear: straight up from where he hangs, then across the lip
            // to the spot he stands on.
            let free = self.region_clear(b.x, b.x, b.y, top)
                && self.region_clear(b.x.min(nx), b.x.max(nx), top, top);
            if !free {
                continue;
            }
            let (cx, cy) = (nx + b.w / 2.0, top + 1.0);
            let from = (b.x, b.y);
            let b = &mut self.p.b;
            b.vx = 0.0;
            b.vy = 0.0;
            self.p.cut = false;
            self.p.mantle = Some(Mantle {
                t: 0.0,
                from,
                to: (nx, top),
            });
            self.cap(cx, cy, Cap::Heave);
            return;
        }
    }

    /// Carries Ben along a pull-up in progress. Gravity and collision do not act; the path was
    /// checked clear when it began, and a tile that has since closed on it (a switched gate) ends
    /// the mantle where he hangs instead of pulling him through it. On the last tick he stands on
    /// the ledge, at rest.
    fn tick_mantling(&mut self, dt: f64) {
        let Some(mut m) = self.p.mantle else { return };
        m.t += dt;
        let done = m.t >= MANTLE_TIME - 1e-9;
        let (x, y) = if done { m.to } else { m.at(m.t) };
        if !self.body_clear(x, y) {
            self.p.mantle = None;
            return;
        }
        let b = &mut self.p.b;
        b.x = x;
        b.y = y;
        b.vx = 0.0;
        b.vy = 0.0;
        if done {
            b.on_ground = true;
            self.p.cut = false;
            self.p.mantle = None;
        } else {
            self.p.mantle = Some(m);
        }
    }

    /// Grabs, moves along and lets go of ladders and vines. Up grabs a ladder at chest height; Down grabs the
    /// ladder under a ledge Ben stands on. Jump lets go with a hop; running out of ladder lets go.
    fn tick_climb(&mut self, h: u32, e: u32, ax: f64) {
        let (up, down) = (h & UP != 0, h & DOWN != 0);
        let cx = self.p.b.centre_x().floor() as i32;
        let chest = (self.p.b.y + 0.5).floor() as i32;
        let ladder = self.map.has_ladder(cx, chest);
        let vine = self.map.has_climb(cx, chest);
        if !self.p.climb {
            if self.p.no_grab {
                if (!up && !down) || !(ladder || vine) {
                    self.p.no_grab = false;
                } else {
                    return;
                }
            }
            if up == down {
                return;
            }
            let from_top = down
                && self.p.b.on_ground
                && self.map.has_ladder(cx, (self.p.b.y - 0.1).floor() as i32);
            if !(up && (ladder || vine)) && !from_top {
                return;
            }
            // A ladder wins where both are present; a vine alone is climbed freely, without
            // snapping Ben to its column.
            self.p.wall = vine && !ladder && !from_top;
            let free = self.p.wall;
            let b = &mut self.p.b;
            if !free {
                b.x = f64::from(cx) + 0.5 - b.w / 2.0;
            }
            if from_top {
                b.y = f64::from((b.y - 0.1).floor() as i32) + 0.45;
            }
            b.vx = 0.0;
            b.vy = 0.0;
            b.on_ground = false;
            self.p.pogo = false;
            self.p.climb = true;
            let (x, y) = (self.p.b.x, self.p.b.y);
            self.cap(x, y + 1.0, Cap::Clink);
            return;
        }
        if e & JUMP != 0 {
            let p = &mut self.p;
            p.climb = false;
            p.wall = false;
            p.no_grab = true;
            p.b.vy = 12.0;
            p.b.vx = ax * 5.0;
            p.cut = true;
            p.b.on_ground = false;
            return;
        }
        let here = if self.p.wall { ladder || vine } else { ladder };
        if !here {
            if !self.p.wall {
                // Off the top of the ladder: settle onto its ledge rather than dropping past it.
                let below = chest - 1;
                let ledge = self.map.props.flags(self.map.get(cx, below)) & ONEWAY != 0;
                if self.p.b.vy > 0.0 && self.map.has_ladder(cx, below) && ledge {
                    self.p.b.y = f64::from(below) + 1.0;
                    self.p.b.vy = 0.0;
                    self.p.b.on_ground = true;
                }
            }
            self.p.climb = false;
            self.p.wall = false;
            return;
        }
        if self.p.b.on_ground && down {
            self.p.climb = false;
            self.p.wall = false;
        }
    }

    fn fire(&mut self) {
        let (px, py) = (self.p.b.x, self.p.b.y);
        if self.game.ammo <= 0 {
            self.cap(px + 0.35, py + 1.6, Cap::NoFizz);
            return;
        }
        self.game.ammo -= 1;
        self.hud();
        let p = &self.p;
        let (mut vx, mut vy) = (p.face * 16.0, 0.0);
        let (mut sx, mut sy) = (p.b.x + p.b.w / 2.0 + p.face * 0.6, p.b.y + 0.85);
        let mut aim = 0;
        if self.held & UP != 0 {
            (vx, vy, sx, sy) = (0.0, 16.0, p.b.x + p.b.w / 2.0, p.b.y + 1.5);
            aim = 1;
        } else if self.held & DOWN != 0 && (!p.b.on_ground || p.climb) {
            (vx, vy, sx, sy) = (0.0, -16.0, p.b.x + p.b.w / 2.0, p.b.y);
            aim = -1;
        }
        self.p.aim = aim;
        self.shots.push(Shot {
            x: sx,
            y: sy,
            vx,
            vy,
            ben: true,
            life: 1.1,
            sprite: Spr::Bubble as u16,
            g: false,
            last: -1,
        });
        self.p.shoot_t = 0.25;
        self.cap(sx, sy + 0.5, Cap::Fzzt);
    }

    /// Kills Ben with no cause recorded (the cause stays 0).
    pub fn kill(&mut self) {
        self.kill_by(0, 0);
    }

    /// Kills Ben and records how, for the shell to read. The record changes nothing in the simulation.
    pub fn kill_by(&mut self, cause: u8, detail: u16) {
        if self.p.dead > 0.0 {
            return;
        }
        let p = &mut self.p;
        p.death_cause = cause;
        p.death_detail = detail;
        p.dead = 0.001;
        p.mantle = None;
        p.b.vy = 14.0;
        p.b.vx = -p.face * 2.0;
        p.pogo = false;
        self.shake = 0.6;
        let (x, y) = (self.p.b.x, self.p.b.y + 1.6);
        self.cap(x, y, Cap::Whoa);
    }

    fn on_death(&mut self) {
        self.game.lives -= 1;
        self.hud();
        if self.game.lives < 0 {
            self.events
                .emit(ev::GAME_OVER, f64::from(self.game.score), 0.0, 0.0);
        } else {
            self.events.emit(
                ev::DIED,
                f64::from(self.game.lives),
                f64::from(self.level_id),
                0.0,
            );
        }
    }

    // ---------- Enemies ----------

    fn tick_ents(&mut self, dt: f64) {
        for i in 0..self.ents.len() {
            let mut e = self.ents[i].clone();
            e.b.px = e.b.x;
            e.b.py = e.b.y;
            e.t += dt;
            if e.dead || info(e.kind).prop {
                self.ents[i] = e;
                continue;
            }
            if e.kind == Kind::Boss {
                self.ents[i] = e;
                self.tick_boss(i, dt);
                continue;
            }
            // Whatever waits in a hidden room stays put until Ben walks in and the flat fades, so
            // an ambush is still there when he arrives instead of having wandered out already.
            let (cx, cy) = (e.b.x + e.b.w / 2.0, e.b.y + e.b.h / 2.0);
            if let Some(r) = self.room_at(cx.floor() as i32, cy.floor() as i32) {
                if self.room_alpha[r] > ROOM_DORMANT_ALPHA {
                    self.ents[i] = e;
                    continue;
                }
            }
            if e.ride {
                // A rider moves with the platform it stands on, as Ben does.
                if let Some(i) = e.b.on_plat {
                    e.b.x += self.plats[i].dx;
                    e.b.y += self.plats[i].dy;
                }
            }
            self.ai(&mut e, dt);
            if !e.ride
                && matches!(
                    e.kind,
                    Kind::Gloop | Kind::Hopper | Kind::Beetle | Kind::Marsh
                )
            {
                // An enemy that has ended up on another floor for a few seconds goes back home.
                // Riders leave their floor on purpose, so they are exempt.
                if (e.b.y - e.ay).abs() > STRAY_HEIGHT {
                    e.stray += dt;
                } else {
                    e.stray = 0.0;
                }
                if e.stray > STRAY_SECS {
                    let (x, y) = (e.b.x + e.b.w / 2.0, e.b.y + 0.5);
                    self.fx.push(Fx {
                        x,
                        y,
                        t: 0.0,
                        life: 0.6,
                        tint: 0xffffff,
                    });
                    self.cap(x, y + 0.8, Cap::Poof);
                    (e.b.x, e.b.y, e.b.vx, e.b.vy) = (e.ax, e.ay, 0.0, 0.0);
                    (e.b.px, e.b.py) = (e.ax, e.ay);
                    e.stray = 0.0;
                    e.stun = 0.0;
                    e.dir = -1.0;
                }
            }
            self.ents[i] = e;
        }
    }

    fn ai(&mut self, e: &mut Ent, dt: f64) {
        let d = info(e.kind);
        if e.stun > 0.0 {
            e.stun -= dt;
            if !d.fly {
                e.b.vx = 0.0;
                e.b.fall(dt, 20.0);
                e.b.phys(&self.map, if e.ride { &self.plats } else { &[] }, dt);
            } else if e.kind == Kind::Bat {
                e.b.move_y(&self.map, -4.0 * dt);
            }
            return;
        }
        let (pcx, py) = (self.p.b.x + self.p.b.w / 2.0, self.p.b.y);
        let dx = pcx - (e.b.x + e.b.w / 2.0);
        let dy = py - e.b.y;
        let ben = self.p.dead == 0.0 && !self.p.hidden;
        match e.kind {
            Kind::Gloop => {
                e.b.vx = e.dir * 1.5;
                e.b.fall(dt, 20.0);
                e.b.phys(&self.map, if e.ride { &self.plats } else { &[] }, dt);
                if e.b.hit_x || (e.b.on_ground && !ground_ahead(&self.map, e)) {
                    e.dir *= -1.0;
                }
            }
            Kind::Hopper => {
                e.b.fall(dt, 20.0);
                e.cd -= dt;
                let behind = sign(e.b.x - self.p.b.x) == -self.p.face;
                if e.b.on_ground {
                    e.b.vx = 0.0;
                    if ben && behind && dx.abs() < 10.0 && dy.abs() < 4.0 && e.cd <= 0.0 {
                        e.dir = sign_or1(dx);
                        e.b.vy = 13.0;
                        e.b.vx = e.dir * 5.5;
                        e.cd = 0.55;
                    }
                }
                e.b.phys(&self.map, if e.ride { &self.plats } else { &[] }, dt);
            }
            Kind::Marsh => {
                e.b.vx = e.dir * 2.2;
                e.b.fall(dt, 20.0);
                e.b.phys(&self.map, if e.ride { &self.plats } else { &[] }, dt);
                if e.b.on_ground {
                    e.b.vy = 11.0;
                }
                if e.b.hit_x {
                    e.dir *= -1.0;
                }
            }
            Kind::Beetle => {
                e.b.fall(dt, 20.0);
                if e.state == St::Charge {
                    e.b.vx = e.dir * 9.0;
                    e.st -= dt;
                    e.b.phys(&self.map, if e.ride { &self.plats } else { &[] }, dt);
                    if e.b.hit_x {
                        e.state = St::Idle;
                        e.stun = 1.2;
                        let (x, y) = (e.b.x + 0.5, e.b.y + 1.0);
                        self.cap(x, y, Cap::Thunk);
                    } else if e.st <= 0.0 || (e.b.on_ground && !ground_ahead(&self.map, e)) {
                        e.state = St::Idle;
                        e.b.vx = 0.0;
                    }
                } else {
                    e.b.vx = 0.0;
                    e.b.phys(&self.map, if e.ride { &self.plats } else { &[] }, dt);
                    if ben && dy.abs() < 1.2 && dx.abs() < 11.0 {
                        e.state = St::Charge;
                        e.dir = sign_or1(dx);
                        e.st = 2.2;
                        let (x, y) = (e.b.x + 0.5, e.b.y + 1.2);
                        self.cap(x, y, Cap::Snort);
                    }
                }
            }
            Kind::Bat => match e.state {
                St::Idle => {
                    e.b.x = e.ax;
                    e.b.y = e.ay;
                    if ben && dx.abs() < 2.4 && dy < -0.5 && dy > -11.0 {
                        e.state = St::Drop;
                        e.b.vx = sign(dx) * 2.0;
                        let (x, y) = (e.b.x, e.b.y);
                        self.cap(x, y, Cap::Skreee);
                    }
                }
                St::Drop => {
                    e.b.move_x(&self.map, e.b.vx * dt, false);
                    let landed = e.b.move_y(&self.map, -15.0 * dt);
                    if landed || e.b.y <= py - 0.2 || e.b.y < 0.0 {
                        e.state = St::Rise;
                    }
                }
                _ => {
                    e.b.x += (e.ax - e.b.x) * (dt * 2.0).min(1.0);
                    e.b.move_y(&self.map, 6.0 * dt);
                    if e.b.y >= e.ay - 0.02 || e.b.bonk {
                        e.state = St::Idle;
                    }
                }
            },
            Kind::Pod => {
                let c = e.t % 3.0;
                let was = e.puff;
                e.puff = c > 2.0;
                if e.puff && !was {
                    let (x, y) = (e.b.x + 0.4, e.b.y + 1.4);
                    self.cap(x, y, Cap::Pfff);
                }
            }
            Kind::Phantom => {
                e.b.fall(dt, 20.0);
                e.b.vx = 0.0;
                e.b.phys(&self.map, if e.ride { &self.plats } else { &[] }, dt);
                e.dir = sign_or1(dx);
                e.cd -= dt;
                e.fire_t -= dt;
                if e.alpha < 1.0 {
                    e.alpha = (e.alpha + dt * 2.0).min(1.0);
                }
                if ben && dx.abs() < 12.0 && dy.abs() < 6.0 && e.fire_t <= 0.0 {
                    e.fire_t = 2.6;
                    let a0 = (dy + 0.5).atan2(dx);
                    for o in [-0.3, 0.0, 0.3] {
                        self.shots.push(Shot {
                            x: e.b.x + e.b.w / 2.0,
                            y: e.b.y + 0.9,
                            vx: (a0 + o).cos() * 6.0,
                            vy: (a0 + o).sin() * 6.0,
                            ben: false,
                            life: 3.0,
                            sprite: Spr::Zshot as u16,
                            g: false,
                            last: -1,
                        });
                    }
                    let (x, y) = (e.b.x + 0.4, e.b.y + 2.0);
                    self.cap(x, y, Cap::ZapZapZap);
                }
            }
            Kind::Roller => {
                e.b.fall(dt, 20.0);
                let sl = e.b.on_slope;
                let down = if sl == R45 || sl == R22A || sl == R22B {
                    -1.0
                } else if sl != 0 {
                    1.0
                } else {
                    0.0
                };
                let acc = if sl == R45 || sl == L45 { 14.0 } else { 7.0 };
                if down != 0.0 {
                    e.b.vx += down * acc * dt;
                } else if e.b.vx.abs() < 2.5 {
                    let s = sign(e.b.vx);
                    e.b.vx = (if s == 0.0 { e.dir } else { s }) * 2.5;
                }
                e.b.vx = e.b.vx.clamp(-8.5, 8.5);
                let vx = e.b.vx;
                e.b.phys(&self.map, if e.ride { &self.plats } else { &[] }, dt);
                if e.b.hit_x {
                    e.b.vx = -vx * 0.6;
                    let s = sign(e.b.vx);
                    e.dir = if s == 0.0 { -e.dir } else { s };
                    let (x, y) = (e.b.x + 1.0, e.b.y + 2.2);
                    self.cap(x, y, Cap::Krunch);
                    self.shake = self.shake.max(0.3);
                }
                e.rot -= e.b.vx * dt / 0.95;
            }
            Kind::Sentry => {
                let ty = (py + 0.3).clamp(e.ay - 4.0, e.ay + 4.0);
                e.b.y += ((ty - e.b.y) * 2.0).clamp(-2.5, 2.5) * dt;
                e.b.x = e.ax + (e.t * 0.9).sin() * 0.8;
                e.dir = sign_or1(dx);
                e.fire_t -= dt;
                if ben && dx.abs() < 9.0 && e.fire_t <= 0.0 {
                    e.fire_t = 1.8;
                    for o in [-0.2f64, 0.0, 0.2] {
                        self.shots.push(Shot {
                            x: e.b.x + e.b.w / 2.0 + e.dir * 0.8,
                            y: e.b.y + 0.4,
                            vx: o.cos() * 7.0 * e.dir,
                            vy: o.sin() * 7.0,
                            ben: false,
                            life: 2.5,
                            sprite: Spr::Zshot as u16,
                            g: false,
                            last: -1,
                        });
                    }
                    let (x, y) = (e.b.x + 0.7, e.b.y + 1.4);
                    self.cap(x, y, Cap::Brrrt);
                }
            }
            Kind::Drone => {
                e.b.x = e.ax + 2.6 * (e.t * 1.6).sin();
                e.b.y = e.ay + 1.4 * (e.t * 3.2).sin();
            }
            Kind::Press => {
                // Raised by default; the phase comes from the spawn position, so a row of presses
                // is staggered the same way every run. Contact kills while it is down.
                e.b.y = e.ay - 3.0 * (0.5 - 0.5 * (e.t * 1.6).cos());
            }
            Kind::Boss
            | Kind::Switch
            | Kind::Terminal
            | Kind::Cage
            | Kind::Mirror
            | Kind::Swivel
            | Kind::CrystalSwitch
            | Kind::Target
            | Kind::Glyph
            | Kind::Cameo => {}
        }
    }

    fn tick_boss(&mut self, i: usize, dt: f64) {
        let a = self.arena.unwrap_or(Arena {
            x0: 0.0,
            x1: 40.0,
            floor: 4.0,
        });
        let floor = a.floor;
        let (pcx, pdead, px) = (self.p.b.x + self.p.b.w / 2.0, self.p.dead > 0.0, self.p.b.x);
        let mut e = self.ents[i].clone();
        let dx = pcx - (e.b.x + e.b.w / 2.0);
        e.st += dt;
        match e.state {
            St::Wait => {
                if px > a.x0 + 3.0 && !pdead {
                    e.state = St::Rise;
                    e.st = 0.0;
                    self.events.emit(ev::DIALOGUE, 0.0, 0.0, 0.0);
                }
            }
            St::Hover => {
                e.b.y += ((floor + 4.5 + (e.t * 2.0).sin() * 0.4) - e.b.y) * (dt * 3.0).min(1.0);
                e.b.x += (dx * 2.0).clamp(-3.0, 3.0) * dt;
                e.b.x = e.b.x.clamp(a.x0, a.x1 - e.b.w);
                if (e.st / 0.9).floor() != ((e.st - dt) / 0.9).floor() {
                    self.shots.push(Shot {
                        x: e.b.x + e.b.w / 2.0,
                        y: e.b.y,
                        vx: 0.0,
                        vy: -1.0,
                        ben: false,
                        life: 4.0,
                        sprite: Spr::Glob as u16,
                        g: true,
                        last: -1,
                    });
                    let (x, y) = (e.b.x + 1.4, e.b.y - 0.3);
                    self.cap(x, y, Cap::Splorp);
                }
                if e.st > 4.2 {
                    e.state = St::Land;
                    e.st = 0.0;
                }
            }
            St::Land => {
                e.b.y -= 9.0 * dt;
                if e.b.y <= floor {
                    e.b.y = floor;
                    e.state = St::Charge;
                    e.st = 0.0;
                    e.dir = sign_or1(dx);
                    self.shake = 0.8;
                    let (x, y) = (e.b.x + 1.4, e.b.y + 3.0);
                    self.cap(x, y, Cap::Thoom);
                }
            }
            St::Charge => {
                e.b.vx = e.dir * 10.0;
                let before = e.b.x;
                e.b.x += e.b.vx * dt;
                if e.b.x < a.x0 || e.b.x + e.b.w > a.x1 {
                    e.b.x = before.clamp(a.x0, a.x1 - e.b.w);
                    e.state = St::Hot;
                    e.st = 0.0;
                    self.shake = 0.5;
                    let (x, y) = (e.b.x + 1.4, e.b.y + 3.0);
                    self.cap(x, y, Cap::Clang);
                }
            }
            St::Hot => {
                if e.st > 2.4 {
                    e.state = St::Rise;
                    e.st = 0.0;
                }
            }
            St::Rise => {
                e.b.y += 5.0 * dt;
                if e.b.y >= floor + 4.5 {
                    e.state = St::Hover;
                    e.st = 0.0;
                }
            }
            St::Down => {
                e.b.y = (e.b.y - 6.0 * dt).max(floor);
                if self.rng.next_f64() < 0.3 {
                    let (rx, ry) = (self.rng.next_f64(), self.rng.next_f64());
                    self.fx.push(Fx {
                        x: e.b.x + rx * e.b.w,
                        y: e.b.y + ry * e.b.h,
                        t: 0.0,
                        life: 0.5,
                        tint: 0xffff55,
                    });
                }
                if e.st > 2.0 {
                    e.dead = true;
                    self.items.push(Item {
                        kind: ItemKind::Usb,
                        x: e.b.x + e.b.w / 2.0,
                        y: floor + 0.6,
                        taken: false,
                    });
                    self.events.emit(ev::DIALOGUE, 1.0, 0.0, 0.0);
                }
            }
            St::Idle | St::Drop => {}
        }
        self.ents[i] = e;
    }

    #[cfg(test)]
    pub fn hit_boss_for_test(&mut self, i: usize) {
        self.hit_boss(i);
    }

    fn hit_boss(&mut self, i: usize) {
        let (x, y) = (self.ents[i].b.x + 1.4, self.ents[i].b.y + 3.2);
        self.ents[i].hp -= 1;
        self.cap(x, y, Cap::Zzzap);
        self.shake = 0.5;
        let hp = self.ents[i].hp;
        self.events.emit(ev::BOSS_HP, f64::from(hp), 0.0, 0.0);
        self.ents[i].st = 0.0;
        self.ents[i].state = if hp <= 0 { St::Down } else { St::Rise };
    }

    fn tick_shots(&mut self, dt: f64) {
        let mut i = self.shots.len();
        while i > 0 {
            i -= 1;
            let mut b = self.shots[i];
            // Where the bubble was at the start of the tick: a bubble fired from right beside a
            // mirror is already moving away from it by the time it is first tested.
            let (ox, oy) = (b.x, b.y);
            if b.g {
                b.vy -= 20.0 * dt;
            }
            b.x += b.vx * dt;
            b.y += b.vy * dt;
            b.life -= dt;
            if b.ben && self.map.get(b.x.floor() as i32, b.y.floor() as i32) == CRACKED {
                self.crumble(b.x.floor() as i32, b.y.floor() as i32);
                b.life = 0.0;
            }
            let mut gone = b.life <= 0.0
                || self
                    .map
                    .solid(b.x.floor() as i32, b.y.floor() as i32, false, 0.0)
                || b.y < 0.0;
            if b.ben && !gone {
                for j in 0..self.ents.len() {
                    let e = &self.ents[j];
                    let d = info(e.kind);
                    if e.dead {
                        continue;
                    }
                    if matches!(e.kind, Kind::Mirror | Kind::Swivel | Kind::CrystalSwitch) {
                        let (kind, dir) = (e.kind, e.dir);
                        let (cx, cy) = (e.b.x + e.b.w / 2.0, e.b.y + e.b.h / 2.0);
                        let reach = if kind == Kind::CrystalSwitch {
                            SWITCH_REACH
                        } else {
                            MIRROR_REACH
                        };
                        let near =
                            |x: f64, y: f64| (x - cx).abs() < reach && (y - cy).abs() < reach;
                        if near(b.x, b.y) || near(ox, oy) {
                            if kind == Kind::CrystalSwitch {
                                // The switch's `dir` holds the channel of the gate it opens.
                                let ch = dir as u8;
                                if self.map.switch(ch) {
                                    self.map.set_switch(ch, false);
                                    self.toast(Toast::GateOpen);
                                }
                                self.cap(cx, cy + 0.7, Cap::Chime);
                                gone = true;
                                break;
                            }
                            if b.last != j as i32 {
                                // A 45 degree mirror swaps the shot's axes: `/` sends up to the right,
                                // `\` sends up to the left.
                                (b.vx, b.vy) = if dir > 0.0 {
                                    (b.vy, b.vx)
                                } else {
                                    (-b.vy, -b.vx)
                                };
                                (b.x, b.y) = (cx, cy);
                                b.life = 1.1;
                                b.last = j as i32;
                                if kind == Kind::Swivel {
                                    self.ents[j].dir = -dir;
                                }
                                self.cap(cx, cy + 0.7, Cap::Ting);
                                self.fx.push(Fx {
                                    x: cx,
                                    y: cy,
                                    t: 0.0,
                                    life: 0.2,
                                    tint: 0x55ffff,
                                });
                            }
                        }
                        continue;
                    }
                    if e.kind == Kind::Target {
                        let (cx, cy) = (e.b.x + e.b.w / 2.0, e.b.y + e.b.h / 2.0);
                        if (b.x - cx).abs() < 0.7 && (b.y - cy).abs() < 0.7 {
                            let n = if e.dir < 1.0 { 3 } else { e.dir as i32 };
                            self.ents[j].dead = true;
                            for k in 0..n {
                                self.items.push(Item {
                                    kind: ItemKind::Cheezie,
                                    x: cx + (f64::from(k) - f64::from(n - 1) / 2.0) * 0.7,
                                    y: cy - 0.2,
                                    taken: false,
                                });
                            }
                            self.cap(cx, cy + 0.7, Cap::Pop);
                            gone = true;
                            break;
                        }
                        continue;
                    }
                    if d.prop {
                        continue;
                    }
                    let (ex, ey, ew, eh) = (e.b.x, e.b.y, e.b.w, e.b.h);
                    if e.kind == Kind::Phantom
                        && e.cd <= 0.0
                        && (b.x - (ex + ew / 2.0)).abs() < 2.6
                        && (b.y - (ey + 0.7)).abs() < 1.2
                        && e.stun <= 0.0
                    {
                        self.teleport_phantom(j);
                        continue;
                    }
                    let inside = |x: f64, y: f64| {
                        x > ex - 0.15
                            && x < ex + ew + 0.15
                            && y > ey - 0.2
                            && y < ey + eh.max(1.1) + 0.2
                    };
                    if inside(b.x, b.y) {
                        // Pass-through rule: a fizz shot flies straight through an enemy that is
                        // already stunned (`stun > 0`). It is not consumed and does not refresh
                        // the stun, so it carries on to the next enemy or the wall. Bosses and
                        // invulnerable enemies still stop it. A shot spawned already overlapping a
                        // stunned enemy simply passes, with no spark. A tiny spark marks the moment
                        // the shot enters the enemy, so the pass reads as intentional.
                        if e.stun > 0.0 && e.kind != Kind::Boss && !d.inv {
                            if !inside(ox, oy) {
                                self.fx.push(Fx {
                                    x: b.x,
                                    y: b.y,
                                    t: 0.0,
                                    life: 0.15,
                                    tint: 0xaaffff,
                                });
                            }
                            continue;
                        }
                        gone = true;
                        let (kind, state) = (e.kind, e.state);
                        if kind == Kind::Boss {
                            if state == St::Hot {
                                self.hit_boss(j);
                            } else {
                                self.cap(b.x, b.y + 0.5, Cap::Plink);
                            }
                        } else if d.inv {
                            self.cap(
                                b.x,
                                b.y + 0.5,
                                if kind == Kind::Marsh {
                                    Cap::Blorp
                                } else {
                                    Cap::Plink
                                },
                            );
                        } else {
                            self.ents[j].stun = 6.0;
                            self.ents[j].state = St::Idle;
                            self.cap(ex + ew / 2.0, ey + eh + 0.4, Cap::Fizzled);
                        }
                        break;
                    }
                }
            } else if !b.ben && !gone && self.p.dead == 0.0 {
                let p = &self.p.b;
                if b.x > p.x && b.x < p.x + p.w && b.y > p.y && b.y < p.y + p.h {
                    gone = true;
                    if self.p.inv <= 0.0 {
                        self.kill_by(death::SHOT, 0);
                    }
                }
            }
            if gone {
                self.fx.push(Fx {
                    x: b.x,
                    y: b.y,
                    t: 0.0,
                    life: 0.3,
                    tint: if b.ben { 0x55ffff } else { 0xff5555 },
                });
                self.shots.swap_remove(i);
            } else {
                self.shots[i] = b;
            }
        }
    }

    /// Breaks the cracked tile at `(x, y)` and every cracked tile joined to it.
    fn crumble(&mut self, x: i32, y: i32) {
        let mut open = vec![(x, y)];
        let mut first = true;
        while let Some((cx, cy)) = open.pop() {
            if self.map.get(cx, cy) != CRACKED {
                continue;
            }
            self.map.set(cx, cy, EMPTY);
            self.fx.push(Fx {
                x: f64::from(cx) + 0.5,
                y: f64::from(cy) + 0.5,
                t: 0.0,
                life: 0.5,
                tint: 0xaaaaaa,
            });
            if first {
                self.cap(f64::from(cx) + 0.5, f64::from(cy) + 1.0, Cap::Crumble);
                first = false;
            }
            open.extend([(cx + 1, cy), (cx - 1, cy), (cx, cy + 1), (cx, cy - 1)]);
        }
    }

    fn teleport_phantom(&mut self, j: usize) {
        let p = self.p.b.clone();
        for tries in 0..6i32 {
            let dir = if hashf(self.tick_count as i32, tries) < 0.5 {
                -1.0
            } else {
                1.0
            };
            let nx = (p.x + dir * f64::from(5 + tries % 3)).floor() as i32;
            let mut y = (self.map.h - 3).min(p.y.floor() as i32 + 4);
            while y > 0 {
                let m = &self.map;
                if m.solid(nx, y - 1, false, 0.0)
                    && !m.solid(nx, y, false, 0.0)
                    && !m.solid(nx, y + 1, false, 0.0)
                    && !is_liquid(m.get(nx, y))
                    && !is_liquid(m.get(nx, y - 1))
                {
                    let (ex, ey) = (self.ents[j].b.x, self.ents[j].b.y);
                    self.fx.push(Fx {
                        x: ex + 0.4,
                        y: ey + 0.7,
                        t: 0.0,
                        life: 0.6,
                        tint: 0xffffff,
                    });
                    let e = &mut self.ents[j];
                    e.b.x = f64::from(nx) + 0.1;
                    e.b.y = f64::from(y);
                    e.b.px = e.b.x;
                    e.b.py = e.b.y;
                    e.cd = 2.4;
                    e.alpha = 0.0;
                    e.fire_t = e.fire_t.min(0.8);
                    let (x, y) = (e.b.x + 0.4, e.b.y + 2.0);
                    self.cap(x, y, Cap::Poof);
                    return;
                }
                y -= 1;
            }
        }
    }
}
