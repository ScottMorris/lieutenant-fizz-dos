// Sound-cue captions and toast text the Episode 1 simulation can emit.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Sound-cue captions and toasts the sim can emit. Captions double as sound-effect triggers
//! (a caption with colour 0 is silent on screen but still triggers its sound).

#[allow(unused_imports)]
use alloc::{boxed::Box, vec, vec::Vec};
#[allow(unused_imports)]
use lf_sim::fmath::{FloatMath, FloatMath32};
macro_rules! captions {
    ($($id:ident = ($text:expr, $col:expr)),* $(,)?) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        #[repr(u16)]
        pub enum Cap { $($id),* }
        pub const CAPTIONS: &[(&str, u32)] = &[$(($text, $col)),*];
    };
}

captions! {
    Jump = ("jump", 0), Boing = ("boing", 0xffff55), Fzzt = ("fzzt", 0x55ffff),
    NoFizz = ("click — no fizz", 0xaaaaaa), Whoa = ("whoa!", 0xff5555), TaDa = ("ta-da!", 0x55ff55),
    Clunk = ("clunk", 0xffffff), Crunch = ("crunch", 0xffff55), ExtraLife = ("ding! extra life", 0x55ff55),
    Fsssht = ("fsssht", 0x55ffff), RedGumdrop = ("red gumdrop", 0xff5555), BlueGumdrop = ("blue gumdrop", 0x5555ff),
    GreenGumdrop = ("green gumdrop", 0x55ff55),
    GoldUsb = ("gold USB drive", 0xffff55), ClickClack = ("click-clack", 0xaaaaaa), BeepBoop = ("beep boop", 0x55ff55),
    NeedsDrive = ("needs a drive", 0xaaaaaa), Sproing = ("sproing", 0xff55ff), Bwomp = ("bwomp", 0xff55ff),
    Bonk = ("bonk", 0xffffff), Pfff = ("pfff", 0xff55ff), Thunk = ("thunk", 0xffffff), Snort = ("snort", 0xff5555),
    Skreee = ("skreee", 0x55ffff), ZapZapZap = ("zap zap zap", 0xff5555), Krunch = ("KRUNCH", 0xaaaaaa),
    Brrrt = ("brrrt", 0xff5555), Splorp = ("splorp", 0xaa5500), Thoom = ("THOOM", 0xffffff),
    Clang = ("CLANG — dome open!", 0xffff55), Zzzap = ("ZZZAP", 0x55ffff), Plink = ("plink", 0xaaaaaa),
    Blorp = ("blorp", 0xaaaaaa), Fizzled = ("fizzled", 0x55ffff), Poof = ("poof", 0xffffff), Vworp = ("vworp", 0x55ffff),
    Clink = ("clink", 0xaaaaaa), Kick = ("kick", 0xffffff), Heave = ("heave", 0xaaaaaa), Ting = ("ting", 0x55ffff), Chime = ("chime", 0x55ffff),
    Crumble = ("crumble", 0xaaaaaa), Pop = ("pop!", 0xffff55),
}

macro_rules! toasts {
    ($($id:ident = $text:expr),* $(,)?) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        #[repr(u16)]
        pub enum Toast { $($id),* }
        pub const TOASTS: &[&str] = &[$($text),*];
    };
}

toasts! {
    RedDoor = "The red cookie door swings open",
    BlueDoor = "The blue cookie door swings open",
    GreenDoor = "The green cookie door swings open",
    ExtraLife = "Extra life! Every 100 snack points earns one.",
    UsbFound = "Gold USB drive! Find the security terminal.",
    BridgeOn = "Somewhere ahead, a bridge rumbles into place",
    BridgeOff = "The bridge folds away",
    GateOpen = "A crystal chimes, and the gate above slides open",
    GlyphGrowth = "A painted wall: tiny figures tend rows of glowing crystals, and a sun with two faces looks down",
    GlyphVisitors = "A painted wall: a round ship lands, and the little figures wave with four arms each",
    GlyphFizz = "A painted wall: a figure pours something fizzy on a dry crack, and a green sprout bursts out",
    GlyphDeep = "A painted wall: a long stair winds down into the dark, and every step has a small candle",
    SaucerHatch = "The hatch is propped open with a stack of comic books. The saucer hums to itself, waiting for a full crew",
    SaucerGalley = "The galley: a pot of spaghetti and meatballs is still simmering. The engine runs on it, and nobody asks how",
    SaucerBunks = "Two bunks. One is Ben's. The other has a football helmet on the pillow, and nobody has touched it",
    SaucerChart = "A star chart with Zargoth circled twice in red marker, and \"COCOA?!\" written beside it",
    SaucerShelf = "A shelf of souvenirs: a very old gumdrop, a jar of crater dust, and a rubber duck in a tiny helmet",
    SaucerBridge = "The bridge. The captain's chair is a bean bag, and the steering wheel came off a go-kart",
    JokePogo = "Thirty bounces. Ben is not a kangaroo, but he is doing a very good impression",
    JokeMarsh = "Ben has been told that marshmallows are not trampolines. He disagrees",
    JokeKick = "Ben's knees file a formal complaint about all this wall-kicking",
    JokeIdle = "Ben whistles a little tune. The level waits politely",
    CameoBilly = "A chalk drawing of Billy in his helmet. \"FIND ME\", it says, with an arrow pointing at nothing",
    CameoMortimer = "A chalk drawing of Mortimer McMire. Someone has added a moustache. \"WANTED: FOR STEALING COCOA\"",
    SecretFound = "A mural of the crystal forest — a ring of trees glows in the far north-east",
}

/// Event kinds shared with the shell (`events.ts` mirrors these).
pub mod ev {
    pub const CAPTION: u32 = 1;
    pub const HUD: u32 = 2;
    pub const TOAST: u32 = 3;
    pub const LEVEL_COMPLETE: u32 = 4;
    pub const DIED: u32 = 5;
    pub const GAME_OVER: u32 = 6;
    pub const DIALOGUE: u32 = 7;
    pub const BOSS_HP: u32 = 8;
    pub const ENDING: u32 = 9;
    pub const MAP_PROMPT: u32 = 10;
    pub const LEVEL_START: u32 = 11;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_line_up() {
        assert_eq!(CAPTIONS[Cap::Vworp as usize].0, "vworp");
        assert_eq!(TOASTS[Toast::BridgeOff as usize], "The bridge folds away");
    }
}
