// Raw C-ABI entry points that expose the Episode 1 simulation to the shell.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Episode 1 simulation, exported to the TypeScript shell as raw C-ABI functions (no
//! wasm-bindgen). The shell talks to it through linear memory:
//!
//! - an instance buffer (`instance_ptr`, 20 floats per sprite) drawn in one instanced call,
//! - an `out` array of per-frame render facts (camera, light count, clear colour, ...),
//! - light uniform arrays, an event queue, and a small indexed state array for the HUD/saves.

#![no_std]

extern crate alloc;
#[cfg(test)]
extern crate std;

#[allow(unused_imports)]
use alloc::{boxed::Box, vec, vec::Vec};
#[allow(unused_imports)]
use lf_sim::fmath::{FloatMath, FloatMath32};

pub mod ents;
pub mod levels;
pub mod render;
pub mod sprites;
pub mod text;
pub mod tiles;
pub mod world;

#[cfg(test)]
mod playtest;
#[cfg(test)]
mod tests;

use core::ptr::addr_of_mut;
use world::{Mode, World};

static mut WORLD: Option<Box<World>> = None;

/// Access to the single world. The wasm module is single threaded and the shell never
/// re-enters an export while another is running, so a plain static is sound here.
#[allow(clippy::mut_from_ref)]
fn w() -> &'static mut World {
    // SAFETY: single-threaded wasm; exports are never re-entered.
    unsafe { (*addr_of_mut!(WORLD)).get_or_insert_with(|| Box::new(World::new())) }
}

/// Instance stride in floats (engine contract).
#[no_mangle]
pub extern "C" fn stride() -> u32 {
    lf_sim::STRIDE as u32
}

// ---------- Name tables (Rust is the source of truth for sprite and text ids) ----------

fn table(t: u32) -> Vec<&'static str> {
    match t {
        0 => sprites::SPRITE_NAMES.to_vec(),
        1 => text::CAPTIONS.iter().map(|c| c.0).collect(),
        _ => text::TOASTS.to_vec(),
    }
}

#[no_mangle]
pub extern "C" fn table_len(t: u32) -> u32 {
    table(t).len() as u32
}

#[no_mangle]
pub extern "C" fn name_ptr(t: u32, i: u32) -> *const u8 {
    table(t)
        .get(i as usize)
        .map_or(core::ptr::null(), |s| s.as_ptr())
}

#[no_mangle]
pub extern "C" fn name_len(t: u32, i: u32) -> u32 {
    table(t).get(i as usize).map_or(0, |s| s.len() as u32)
}

#[no_mangle]
pub extern "C" fn caption_colour(i: u32) -> u32 {
    text::CAPTIONS.get(i as usize).map_or(0, |c| c.1)
}

/// Pointer to the sprite rect table: 6 floats per sprite (u, v, uw, vh, w, h).
#[no_mangle]
pub extern "C" fn sprite_table_ptr() -> *mut f32 {
    w().spr.as_mut_ptr().cast()
}

// ---------- Buffers ----------

#[no_mangle]
pub extern "C" fn instance_ptr() -> *const f32 {
    w().inst.ptr()
}

#[no_mangle]
pub extern "C" fn instance_capacity() -> u32 {
    w().inst.capacity() as u32
}

#[no_mangle]
pub extern "C" fn out_ptr() -> *const f32 {
    w().out.as_ptr()
}

#[no_mangle]
pub extern "C" fn lights_pos_ptr() -> *const f32 {
    w().lights_pos.as_ptr()
}

#[no_mangle]
pub extern "C" fn lights_col_ptr() -> *const f32 {
    w().lights_col.as_ptr()
}

#[no_mangle]
pub extern "C" fn events_ptr() -> *const f32 {
    w().events.ptr()
}

#[no_mangle]
pub extern "C" fn events_len() -> u32 {
    w().events.len() as u32
}

#[no_mangle]
pub extern "C" fn events_clear() {
    w().events.clear();
}

// ---------- Overworld thumbnail ----------

static mut THUMB: Option<(i32, i32, Vec<u8>)> = None;

/// The overworld as one byte per tile (see `levels::overworld_thumb`), built once.
fn thumb() -> &'static (i32, i32, Vec<u8>) {
    // SAFETY: single-threaded wasm; exports are never re-entered.
    unsafe { (*addr_of_mut!(THUMB)).get_or_insert_with(levels::overworld_thumb) }
}

#[no_mangle]
pub extern "C" fn thumb_w() -> u32 {
    thumb().0 as u32
}

#[no_mangle]
pub extern "C" fn thumb_h() -> u32 {
    thumb().1 as u32
}

#[no_mangle]
pub extern "C" fn thumb_ptr() -> *const u8 {
    thumb().2.as_ptr()
}

/// The overworld area (see the `AREA_*` ids) that covers a tile position.
#[no_mangle]
pub extern "C" fn area_of(x: f64, y: f64) -> u32 {
    u32::from(levels::area_at(
        &levels::overworld_areas(),
        x.floor() as i32,
        y.floor() as i32,
    ))
}

// ---------- Flow ----------

#[no_mangle]
pub extern "C" fn game_new() {
    w().game_new();
}

/// Loads one level of the attract loop; `still` holds the camera for reduced motion.
#[no_mangle]
pub extern "C" fn load_attract(idx: u32, still: u32) {
    w().load_attract(idx, still != 0);
}

#[no_mangle]
pub extern "C" fn enter_level(id: u32) {
    w().enter_level(u8::try_from(id).unwrap_or(u8::MAX));
}

/// A fingerprint of a level's tiles and what is placed in it (see `LevelData::fingerprint`), so a recording
/// can tell whether the level it was made on has changed since. An unknown id gives the first level's.
#[no_mangle]
pub extern "C" fn level_hash(id: u32) -> u32 {
    levels::build_level(u8::try_from(id).unwrap_or(u8::MAX)).fingerprint()
}

/// How many shots are in flight (read-only): Ben's fizz bubbles and the enemies' shots together.
#[no_mangle]
pub extern "C" fn shot_count() -> u32 {
    u32::try_from(w().shots.len()).unwrap_or(u32::MAX)
}

/// One field of shot `i` (read-only): 0 x, 1 y, 2 vx, 3 vy, 4 whether it is Ben's (1) or an enemy's (0).
/// Anything else, or a shot that is not there, reads 0. No rule reads this; a replay uses it to trace fizz.
#[no_mangle]
pub extern "C" fn shot_get(i: u32, field: u32) -> f64 {
    let Some(s) = w().shots.get(i as usize) else {
        return 0.0;
    };
    match field {
        0 => s.x,
        1 => s.y,
        2 => s.vx,
        3 => s.vy,
        4 => f64::from(u8::from(s.ben)),
        _ => 0.0,
    }
}

#[no_mangle]
pub extern "C" fn enter_map() {
    w().enter_map();
}

/// Stops simulating (the shell is drawing a cinematic).
#[no_mangle]
pub extern "C" fn enter_none() {
    w().mode = Mode::None;
}

#[no_mangle]
pub extern "C" fn set_view(half_w: f64, half_h: f64) {
    let s = w();
    s.half_w = half_w;
    s.half_h = half_h;
}

/// Advances one fixed 60 Hz tick with the given input bits.
#[no_mangle]
pub extern "C" fn step(held: u32) {
    w().step(held);
}

/// Writes the scene into the instance buffer; returns the instance count.
#[no_mangle]
pub extern "C" fn render(alpha: f64, flags: u32) -> u32 {
    let s = w();
    s.render(alpha, flags);
    s.inst.len() as u32
}

#[no_mangle]
pub extern "C" fn mode() -> u32 {
    match w().mode {
        Mode::None => 0,
        Mode::Attract => 1,
        Mode::Map => 2,
        Mode::Level => 3,
    }
}

// ---------- Indexed state (HUD, saves) ----------

pub mod state {
    pub const LIVES: u32 = 0;
    pub const SCORE: u32 = 1;
    pub const NEXT_LIFE: u32 = 2;
    pub const AMMO: u32 = 3;
    pub const DONE_MASK: u32 = 4;
    pub const MAP_X: u32 = 5;
    pub const MAP_Y: u32 = 6;
    pub const HAS_MAP_POS: u32 = 7;
    pub const KEY_RED: u32 = 8;
    pub const KEY_BLUE: u32 = 9;
    pub const HAS_USB: u32 = 10;
    pub const BOSS_HP: u32 = 11;
    pub const POGO_HEIGHT: u32 = 12;
    pub const PLAYER_X: u32 = 13;
    pub const PLAYER_Y: u32 = 14;
    pub const PLAYER_DEAD: u32 = 15;
    pub const TICK: u32 = 16;
    pub const LEVEL_ID: u32 = 17;
    pub const WON: u32 = 18;
    pub const KEY_GREEN: u32 = 19;
    pub const ATTRACT_T: u32 = 20;
    pub const ATTRACT_PERIOD: u32 = 21;
    pub const ATTRACT_IDX: u32 = 22;
    /// Whether Ben's pogo toggle is on (read-only; 0 outside a level).
    pub const POGO_ON: u32 = 23;
    /// How Ben last died (read-only): 0 unknown or alive, 1 fell, 2 spikes, 3 chocolate, 4 molten metal,
    /// 5 an enemy, 6 a pod's puff, 7 a shot. No rule reads it.
    pub const DEATH_CAUSE: u32 = 24;
    /// What killed him (read-only): the tile id for spikes, chocolate and molten metal, the enemy kind
    /// (its position in `ents::Kind`) for an enemy or a puff, else 0.
    pub const DEATH_DETAIL: u32 = 25;
}

#[no_mangle]
pub extern "C" fn state_get(i: u32) -> f64 {
    use state::*;
    let s = w();
    let b = |v: bool| f64::from(u8::from(v));
    match i {
        LIVES => f64::from(s.game.lives),
        SCORE => f64::from(s.game.score),
        NEXT_LIFE => f64::from(s.game.next_life),
        AMMO => f64::from(s.game.ammo),
        DONE_MASK => f64::from(s.game.done),
        MAP_X => s.game.map_pos.map_or(0.0, |p| p.0),
        MAP_Y => s.game.map_pos.map_or(0.0, |p| p.1),
        HAS_MAP_POS => b(s.game.map_pos.is_some()),
        KEY_RED => b(s.keys_red),
        KEY_BLUE => b(s.keys_blue),
        KEY_GREEN => b(s.keys_green),
        HAS_USB => b(s.has_usb),
        BOSS_HP => s
            .ents
            .iter()
            .find(|e| e.kind == ents::Kind::Boss)
            .map_or(0.0, |e| f64::from(e.hp)),
        POGO_HEIGHT => s.pogo_height,
        PLAYER_X => s.p.b.x,
        PLAYER_Y => s.p.b.y,
        PLAYER_DEAD => b(s.p.dead > 0.0),
        TICK => f64::from(s.tick_count),
        LEVEL_ID => f64::from(s.level_id),
        WON => b(s.won),
        ATTRACT_T => f64::from(s.attract_t),
        ATTRACT_PERIOD => f64::from(s.attract_period()),
        ATTRACT_IDX => f64::from(s.attract_idx),
        POGO_ON => b(s.p.pogo),
        DEATH_CAUSE => f64::from(s.p.death_cause),
        DEATH_DETAIL => f64::from(s.p.death_detail),
        _ => 0.0,
    }
}

#[no_mangle]
pub extern "C" fn state_set(i: u32, v: f64) {
    use state::*;
    let s = w();
    match i {
        LIVES => s.game.lives = v as i32,
        SCORE => s.game.score = v as i32,
        NEXT_LIFE => s.game.next_life = v as i32,
        AMMO => s.game.ammo = v as i32,
        DONE_MASK => {
            s.game.load_done(v as u32);
            // A flag still waiting to be thrown belongs to the session this replaces.
            s.fresh_clear = None;
            s.flag_throw = None;
        }
        MAP_X => {
            let y = s.game.map_pos.map_or(0.0, |p| p.1);
            s.game.map_pos = Some((v, y));
        }
        MAP_Y => {
            let x = s.game.map_pos.map_or(0.0, |p| p.0);
            s.game.map_pos = Some((x, v));
        }
        HAS_MAP_POS => {
            if v == 0.0 {
                s.game.map_pos = None;
            }
        }
        POGO_HEIGHT => s.pogo_height = v,
        // Restores a recorded tick: a Phantom's teleport reads the absolute tick, so a replay of a level
        // that started late in a session needs the tick it started on.
        TICK => s.tick_count = v as u32,
        // Debug hooks: place Ben and snap the camera to him, so tooling can put him anywhere.
        PLAYER_X => {
            s.p.b.x = v;
            s.p.b.px = v;
            s.p.mantle = None;
            s.cam_x = v + 6.0;
            s.pcx = s.cam_x;
        }
        PLAYER_Y => {
            s.p.b.y = v;
            s.p.b.py = v;
            s.p.mantle = None;
            s.cam_y = v + 3.0;
            s.pcy = s.cam_y;
        }
        _ => {}
    }
}
