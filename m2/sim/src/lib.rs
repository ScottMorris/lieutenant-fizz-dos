// Crate root for the game-agnostic engine core: module wiring and shared constants.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Lieutenant Fizz engine core (game-agnostic).
//!
//! A deterministic fixed-step simulation toolkit: a tile map with per-tile properties
//! (solid, one-way, switchable on numbered channels, conveyor, ladder, 45 and 22.5 degree slopes), an AABB body that resolves X then
//! Y against it, moving platforms, a front-to-back instance buffer shared with the renderer,
//! a nearest-first light selector and a flat event queue.

#![no_std]

extern crate alloc;
#[cfg(test)]
extern crate std;

pub mod body;
pub mod events;
pub mod fmath;
pub mod instance;
pub mod lights;
pub mod platform;
pub mod rng;
pub mod tilemap;

pub use body::Body;
pub use events::{Event, EventQueue};
pub use instance::{InstanceBuffer, PushOpts, SpriteRect};
pub use lights::{Light, LightPool};
pub use platform::Platform;
pub use rng::{hashf, Rng};
pub use tilemap::{slope_height, TileMap, TileProps};

/// Instance stride in floats, shared with the renderer (ENGINE_SPEC §3.3).
pub const STRIDE: usize = 20;
/// Fixed simulation step (60 Hz).
pub const STEP: f64 = 1.0 / 60.0;
/// World gravity in tiles per second squared.
pub const GRAVITY: f64 = 60.0;
