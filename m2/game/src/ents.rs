// Entity, item and player-state types for the Episode 1 world.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[allow(unused_imports)]
use alloc::{boxed::Box, vec, vec::Vec};
#[allow(unused_imports)]
use lf_sim::fmath::{FloatMath, FloatMath32};
use lf_sim::Body;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Gloop,
    Hopper,
    Marsh,
    Beetle,
    Bat,
    Pod,
    Phantom,
    Roller,
    Sentry,
    Drone,
    Boss,
    Switch,
    Terminal,
    Cage,
    /// Fixed 45-degree mirror: bounces fizz. `dir` > 0 is `/`, `dir` < 0 is `\`.
    Mirror,
    /// A mirror that swings to the other angle after every bounce.
    Swivel,
    /// Opens the gate on switch channel 1 when a fizz bubble hits it.
    CrystalSwitch,
    /// A crushing press that rises and falls over a floor; harmless while it is raised.
    Press,
    /// Shootable scenery: a fizz bubble pops it, and it drops `dir` cheezies (3 if unset).
    Target,
    /// Painted wall art: touching it shows a line of lore, chosen by `dir`.
    Glyph,
    /// A chalk drawing of a familiar face: `dir` > 0 is Mortimer, otherwise Billy.
    Cameo,
}

/// Static per-kind traits (size and how the player may interact).
#[derive(Clone, Copy, Debug)]
pub struct Info {
    pub w: f64,
    pub h: f64,
    /// Can be stunned by a stomp or a fizz shot.
    pub stun: bool,
    /// Invincible: shots and stomps do nothing.
    pub inv: bool,
    /// Never hurts on touch.
    pub harmless: bool,
    pub fly: bool,
    /// Scenery or interactable, not an enemy.
    pub prop: bool,
}

const D: Info = Info {
    w: 1.0,
    h: 1.0,
    stun: false,
    inv: false,
    harmless: false,
    fly: false,
    prop: false,
};

pub fn info(k: Kind) -> Info {
    match k {
        Kind::Gloop => Info {
            w: 0.9,
            h: 0.6,
            stun: true,
            ..D
        },
        Kind::Hopper => Info {
            w: 0.9,
            h: 0.9,
            stun: true,
            ..D
        },
        Kind::Marsh => Info {
            w: 0.9,
            h: 0.9,
            inv: true,
            harmless: true,
            ..D
        },
        Kind::Beetle => Info {
            w: 1.0,
            h: 0.75,
            stun: true,
            ..D
        },
        Kind::Bat => Info {
            w: 0.8,
            h: 0.7,
            stun: true,
            fly: true,
            ..D
        },
        Kind::Pod => Info {
            w: 0.8,
            h: 1.0,
            stun: true,
            harmless: true,
            ..D
        },
        Kind::Phantom => Info {
            w: 0.8,
            h: 1.4,
            stun: true,
            ..D
        },
        Kind::Roller => Info {
            w: 1.9,
            h: 1.9,
            inv: true,
            ..D
        },
        Kind::Sentry => Info {
            w: 1.4,
            h: 0.9,
            stun: true,
            fly: true,
            ..D
        },
        Kind::Drone => Info {
            w: 0.9,
            h: 0.6,
            inv: true,
            fly: true,
            ..D
        },
        Kind::Boss => Info {
            w: 2.8,
            h: 2.7,
            ..D
        },
        Kind::Switch => Info {
            w: 0.8,
            h: 1.0,
            harmless: true,
            prop: true,
            ..D
        },
        Kind::Terminal => Info {
            w: 1.0,
            h: 1.5,
            harmless: true,
            prop: true,
            ..D
        },
        Kind::Cage => Info {
            w: 1.0,
            h: 1.5,
            harmless: true,
            prop: true,
            ..D
        },
        Kind::Press => Info {
            w: 2.0,
            h: 1.5,
            inv: true,
            ..D
        },
        Kind::Target => Info {
            w: 0.8,
            h: 0.8,
            harmless: true,
            prop: true,
            ..D
        },
        Kind::Cameo => Info {
            w: 1.0,
            h: 1.5,
            harmless: true,
            prop: true,
            ..D
        },
        Kind::Glyph => Info {
            w: 1.6,
            h: 1.4,
            harmless: true,
            prop: true,
            ..D
        },
        Kind::Mirror | Kind::Swivel | Kind::CrystalSwitch => Info {
            w: 1.0,
            h: 1.0,
            harmless: true,
            prop: true,
            ..D
        },
    }
}

/// Behaviour state shared by the enemy state machines.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum St {
    Idle,
    Charge,
    Drop,
    Rise,
    Wait,
    Hover,
    Land,
    Hot,
    Down,
}

/// Spawn description from the level builder (tile coordinates).
#[derive(Clone, Copy, Debug)]
pub struct Spawn {
    pub kind: Kind,
    pub x: f64,
    pub y: f64,
    /// Facing or orientation: -1 or 1. Mirrors use it for `\` and `/`.
    pub dir: f64,
    /// Rides moving platforms instead of passing through them.
    pub ride: bool,
}

#[derive(Clone, Debug)]
pub struct Ent {
    pub kind: Kind,
    pub b: Body,
    pub dir: f64,
    pub stun: f64,
    pub t: f64,
    pub cd: f64,
    pub fire_t: f64,
    pub state: St,
    pub st: f64,
    pub ax: f64,
    pub ay: f64,
    pub rot: f64,
    pub alpha: f64,
    pub hp: i32,
    pub puff: bool,
    pub touch: bool,
    pub dead: bool,
    /// Seconds spent far from the home floor.
    pub stray: f64,
    /// Rides moving platforms (and so is never sent home for straying).
    pub ride: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemKind {
    Cheezie,
    Choc,
    Cookie,
    Soda,
    KeyRed,
    KeyBlue,
    KeyGreen,
    Usb,
}

#[derive(Clone, Copy, Debug)]
pub struct Item {
    pub kind: ItemKind,
    pub x: f64,
    pub y: f64,
    pub taken: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct Shot {
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
    pub ben: bool,
    pub life: f64,
    pub sprite: u16,
    /// Falls under gravity (boss globs).
    pub g: bool,
    /// Index of the mirror this shot last bounced off, so it cannot bounce off it twice in a row.
    pub last: i32,
}

#[derive(Clone, Copy, Debug)]
pub struct Fx {
    pub x: f64,
    pub y: f64,
    pub t: f64,
    pub life: f64,
    pub tint: u32,
}
