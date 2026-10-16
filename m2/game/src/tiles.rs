// Tile id constants and per-tile properties for the Episode 1 maps.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[allow(unused_imports)]
use alloc::{boxed::Box, vec, vec::Vec};
#[allow(unused_imports)]
use lf_sim::fmath::{FloatMath, FloatMath32};
use lf_sim::tilemap::{
    TileProps, CLIMB, CONVEY_L, CONVEY_R, LADDER as LADDER_FLAG, ONEWAY, SLOPE_L22A, SLOPE_L22B,
    SLOPE_L45, SLOPE_R22A, SLOPE_R22B, SLOPE_R45, SOLID, SWITCHED,
};

/// Switch channel that gates use; channel 0 belongs to the bridge.
pub const GATE_CHANNEL: u8 = 1;

pub const EMPTY: u8 = 0;
pub const FILL: u8 = 1;
pub const BLOCK: u8 = 2;
pub const PLAT: u8 = 3;
pub const SPIKE: u8 = 4;
pub const CHOC: u8 = 5;
pub const R45: u8 = 6;
pub const L45: u8 = 7;
pub const R22A: u8 = 8;
pub const R22B: u8 = 9;
pub const L22A: u8 = 10;
pub const L22B: u8 = 11;
pub const DOOR_R: u8 = 12;
pub const DOOR_B: u8 = 13;
/// Green cookie door, opened by the green gumdrop (Frosting Spire).
pub const DOOR_G: u8 = 35;
pub const BRIDGE: u8 = 14;
pub const CRYS: u8 = 16;
pub const EXIT: u8 = 17;
pub const GRASS: u8 = 20;
pub const PATH: u8 = 21;
pub const RIVER: u8 = 22;
pub const TREE: u8 = 23;
pub const ROCK: u8 = 24;
/// Conveyor belts: solid, and carry a body standing on them to the left or the right.
pub const CONV_L: u8 = 25;
pub const CONV_R: u8 = 26;
/// Molten metal: a lethal liquid like chocolate, with its own look.
pub const FURNACE: u8 = 27;
/// A wall mural. Its bottom-left tile draws the whole 3-by-2 picture; the other five do not draw.
pub const MURAL: u8 = 18;
pub const MURAL_PART: u8 = 19;
/// Gates block a shaft while their switch channel is on; a crystal switch turns it off. Each gate
/// has its own channel, so a level can hold several independent puzzles.
pub const GATE: u8 = 32;
pub const GATE_2: u8 = 33;
pub const GATE_3: u8 = 34;
/// Painted flat that hides a room (see `levels::Room`); fades out while Ben is inside it.
pub const FACADE: u8 = 31;
/// A crystal vine on a cave wall: not solid, and Ben can free-climb it in any direction.
pub const VINE: u8 = 36;
/// A cracked wall: solid until a fizz shot hits it, then it (and any cracked tiles touching it)
/// crumbles away, opening a hidden way through.
pub const CRACKED: u8 = 37;
/// Town buildings and a fountain on the overworld: solid, drawn over grass.
pub const HOUSE_A: u8 = 38;
pub const HOUSE_B: u8 = 39;
pub const SHOP: u8 = 40;
pub const FOUNTAIN: u8 = 41;
/// Decorative interior wall behind a building's rooms, stairs and ladders (not solid).
pub const WALLBG: u8 = 30;
/// Climbable rung (not solid).
pub const RUNG: u8 = 28;
/// Top rung of a ladder: climbable, and a one-way ledge Ben can stand on.
pub const RUNG_TOP: u8 = 29;

/// Whether a tile is a lethal liquid (fudge or molten metal).
pub fn is_liquid(t: u8) -> bool {
    t == CHOC || t == FURNACE
}

/// Collision behaviour for every Episode 1 tile id.
pub fn props() -> TileProps {
    let mut p = TileProps::default();
    for t in [
        FILL, BLOCK, CRACKED, DOOR_R, DOOR_B, DOOR_G, RIVER, TREE, ROCK, HOUSE_A, HOUSE_B, SHOP,
        FOUNTAIN,
    ] {
        p.set_flags(t, SOLID);
    }
    p.set_flags(CONV_L, SOLID | CONVEY_L);
    p.set_flags(CONV_R, SOLID | CONVEY_R);
    p.set_flags(PLAT, ONEWAY);
    for (t, ch) in [(GATE, GATE_CHANNEL), (GATE_2, 2), (GATE_3, 3)] {
        p.set_flags(t, SWITCHED);
        p.set_channel(t, ch);
    }
    p.set_flags(VINE, CLIMB);
    p.set_flags(RUNG, LADDER_FLAG);
    p.set_flags(RUNG_TOP, LADDER_FLAG | ONEWAY);
    p.set_flags(BRIDGE, SWITCHED);
    for (t, s) in [
        (R45, SLOPE_R45),
        (L45, SLOPE_L45),
        (R22A, SLOPE_R22A),
        (R22B, SLOPE_R22B),
        (L22A, SLOPE_L22A),
        (L22B, SLOPE_L22B),
    ] {
        p.set_slope(t, s);
    }
    p
}
