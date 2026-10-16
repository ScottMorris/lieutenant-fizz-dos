// Table of sprite ids the Episode 1 simulation draws.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Sprite ids drawn by the sim. The names must exist in the TypeScript sprite set; the shell
//! reads this table at boot and fails loudly if one is missing.

#[allow(unused_imports)]
use alloc::{boxed::Box, vec, vec::Vec};
#[allow(unused_imports)]
use lf_sim::fmath::{FloatMath, FloatMath32};
macro_rules! sprites {
    ($($id:ident = $name:expr),* $(,)?) => {
        /// Sprite id (index into the sprite rect table the shell fills at boot).
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        #[repr(u16)]
        pub enum Spr { $($id),* }
        pub const SPRITE_NAMES: &[&str] = &[$($name),*];
        impl From<Spr> for u16 { fn from(s: Spr) -> u16 { s as u16 } }
    };
}

sprites! {
    BenStand = "ben_stand", BenRun1 = "ben_run1", BenRun2 = "ben_run2", BenJump = "ben_jump",
    BenShoot = "ben_shoot", BenLookUp = "ben_lookUp", BenLookDown = "ben_lookDown", BenAimUp = "ben_aimUp", BenAimDown = "ben_aimDown", BenPogo = "ben_pogo", BenPogo2 = "ben_pogo2",
    BenClimb1 = "ben_climb1", BenClimb2 = "ben_climb2",
    BenMantle1 = "ben_mantle1", BenMantle2 = "ben_mantle2", BenMantle3 = "ben_mantle3",
    BenMap0 = "benMap0", BenMap1 = "benMap1", BillyCage = "billyCage", Billy = "billy", BillyAlt = "billyAlt",
    Gloop0 = "gloop0", Gloop1 = "gloop1", Hopper0 = "hopper0", Hopper1 = "hopper1",
    Marsh0 = "marsh0", Marsh1 = "marsh1", Beetle0 = "beetle0", Beetle1 = "beetle1",
    Bat0 = "bat0", Bat1 = "bat1", Pod0 = "pod0", Pod1 = "pod1",
    Phantom0 = "phantom0", Phantom1 = "phantom1", Sentry0 = "sentry0", Sentry1 = "sentry1",
    Drone0 = "drone0", Drone1 = "drone1", Boss0 = "boss0", Boss1 = "boss1",
    Spore0 = "spore0", Spore1 = "spore1", Roller = "roller",
    Bubble = "bubble", Zshot = "zshot", Glob = "glob", Stars0 = "stars0", Stars1 = "stars1", Puff = "puff",
    Cheezie = "cheezie", Choc = "choc", Cookie = "cookie", Soda = "soda",
    KeyRed = "keyRed", KeyBlue = "keyBlue", KeyGreen = "keyGreen", Usb = "usb",
    CraterTop = "craterTop", CraterFill = "craterFill", CraterR45 = "craterR45", CraterL45 = "craterL45",
    CraterR22A = "craterR22A", CraterR22B = "craterR22B", CraterL22A = "craterL22A", CraterL22B = "craterL22B",
    CraterPlat = "craterPlat", CraterBlock = "craterBlock", CraterBack = "craterBack",
    CavesTop = "cavesTop", CavesFill = "cavesFill", CavesR45 = "cavesR45", CavesL45 = "cavesL45",
    CavesR22A = "cavesR22A", CavesR22B = "cavesR22B", CavesL22A = "cavesL22A", CavesL22B = "cavesL22B",
    CavesPlat = "cavesPlat", CavesBlock = "cavesBlock", CavesBack = "cavesBack",
    CitadelTop = "citadelTop", CitadelFill = "citadelFill", CitadelR45 = "citadelR45", CitadelL45 = "citadelL45",
    CitadelR22A = "citadelR22A", CitadelR22B = "citadelR22B", CitadelL22A = "citadelL22A", CitadelL22B = "citadelL22B",
    CitadelPlat = "citadelPlat", CitadelBlock = "citadelBlock", CitadelBack = "citadelBack",
    SkyTop = "skyTop", SkyFill = "skyFill", SkyR45 = "skyR45", SkyL45 = "skyL45",
    SkyR22A = "skyR22A", SkyR22B = "skyR22B", SkyL22A = "skyL22A", SkyL22B = "skyL22B",
    SkyPlat = "skyPlat", SkyBlock = "skyBlock", SkyBack = "skyBack",
    BuildingTop = "buildingTop", BuildingFill = "buildingFill", BuildingR45 = "buildingR45", BuildingL45 = "buildingL45",
    BuildingR22A = "buildingR22A", BuildingR22B = "buildingR22B", BuildingL22A = "buildingL22A", BuildingL22B = "buildingL22B",
    BuildingPlat = "buildingPlat", BuildingBlock = "buildingBlock", BuildingBack = "buildingBack",
    TheatreTop = "theatreTop", TheatreFill = "theatreFill", TheatreR45 = "theatreR45", TheatreL45 = "theatreL45",
    TheatreR22A = "theatreR22A", TheatreR22B = "theatreR22B", TheatreL22A = "theatreL22A", TheatreL22B = "theatreL22B",
    TheatrePlat = "theatrePlat", TheatreBlock = "theatreBlock", TheatreBack = "theatreBack",
    FoundryTop = "foundryTop", FoundryFill = "foundryFill", FoundryR45 = "foundryR45", FoundryL45 = "foundryL45",
    FoundryR22A = "foundryR22A", FoundryR22B = "foundryR22B", FoundryL22A = "foundryL22A", FoundryL22B = "foundryL22B",
    FoundryPlat = "foundryPlat", FoundryBlock = "foundryBlock", FoundryBack = "foundryBack",
    Facade = "facade",
    ConveyL0 = "conveyL0", ConveyL1 = "conveyL1", ConveyR0 = "conveyR0", ConveyR1 = "conveyR1",
    Press0 = "press0", Press1 = "press1",
    FurnTop0 = "furnTop0", FurnTop1 = "furnTop1", FurnDeep0 = "furnDeep0", FurnDeep1 = "furnDeep1",
    CrysC = "crysC", CrysM = "crysM", SpikeTile = "spike",
    ChocTop0 = "chocTop0", ChocTop1 = "chocTop1", ChocDeep0 = "chocDeep0", ChocDeep1 = "chocDeep1",
    DoorRed = "doorRed", DoorBlue = "doorBlue", DoorGreen = "doorGreen", ExitTop = "exitTop", ExitBot = "exitBot",
    Bridge = "bridge", Gate = "gate", Mirror = "mirror", CrysSwitchOff = "crysSwitchOff", CrysSwitchOn = "crysSwitchOn", Hover0 = "hover0", Hover1 = "hover1", Lift0 = "lift0", Lift1 = "lift1",
    Ladder = "ladder", LadderTop = "ladderTop", Vine = "vine", Cracked = "cracked", Target = "target", Glyph = "glyph", SwitchOff = "switchOff", SwitchOn = "switchOn",
    Terminal0 = "terminal0", Terminal1 = "terminal1",
    OwGrass = "owGrass", OwPath = "owPath", OwRiver0 = "owRiver0", OwRiver1 = "owRiver1",
    OwTree0 = "owTree0", OwTree1 = "owTree1", OwRock = "owRock", OwHouseA = "owHouseA", OwHouseB = "owHouseB", OwShop = "owShop", OwFountain = "owFountain", OwSign = "owSign",
    OwGrassMeadow = "owGrassMeadow", OwPuff0 = "owPuff0", OwPuff1 = "owPuff1", OwPuffRock = "owPuffRock",
    OwGrassCandy = "owGrassCandy", OwCandy0 = "owCandy0", OwCandy1 = "owCandy1", OwCandyRock = "owCandyRock",
    OwGrassFrost = "owGrassFrost", OwFrost0 = "owFrost0", OwFrost1 = "owFrost1", OwCake = "owCake", OwCrater = "owCrater",
    OwCave = "owCave", OwMesa = "owMesa", OwTower = "owTower", OwPlayhouse = "owPlayhouse", OwMeadow = "owMeadow", OwShaft = "owShaft", OwFoundry = "owFoundry", OwCastle = "owCastle", OwTele0 = "owTele0", OwTele1 = "owTele1", OwFlag = "owFlag",
    Saucer = "saucer", HillTop = "hillTop", MtnTop = "mtnTop", Fill = "fill", Star = "star", Cloud = "cloud", Mural = "mural",
    OwShadowS = "owShadowS", OwShadowL = "owShadowL",
    OwGrassB = "owGrassB", OwGrassC = "owGrassC", OwGrassMeadowB = "owGrassMeadowB", OwGrassMeadowC = "owGrassMeadowC",
    OwGrassCandyB = "owGrassCandyB", OwGrassCandyC = "owGrassCandyC", OwGrassFrostB = "owGrassFrostB", OwGrassFrostC = "owGrassFrostC",
    OwBank = "owBank", OwLipH = "owLipH", OwLipV = "owLipV", OwGlint = "owGlint",
    OwDecal0 = "owDecal0", OwDecal1 = "owDecal1", OwDecal2 = "owDecal2", OwDecal3 = "owDecal3",
    OwLmCrater = "owLmCrater", OwLmCaves = "owLmCaves", OwLmMesa = "owLmMesa", OwLmLookout = "owLmLookout",
    OwLmMeadows = "owLmMeadows", OwLmFudgeBog = "owLmFudgeBog", OwLmPlayhouse = "owLmPlayhouse",
    OwLmShafts = "owLmShafts", OwLmSugarGlass = "owLmSugarGlass", OwLmFlats = "owLmFlats",
    OwLmSpire = "owLmSpire", OwLmFoundry = "owLmFoundry", OwLmGumdrop = "owLmGumdrop", OwLmHollow = "owLmHollow",
    OwFlagHolder = "owFlagHolder",
}

/// Per-tileset tile sprites are laid out in this order, 11 per tileset, starting at `CraterTop`.
pub const TILESET_STRIDE: u16 = 11;
pub const BT_TOP: u16 = 0;
pub const BT_FILL: u16 = 1;
pub const BT_SLOPE0: u16 = 2; // R45, L45, R22A, R22B, L22A, L22B
pub const BT_PLAT: u16 = 8;
pub const BT_BLOCK: u16 = 9;
pub const BT_BACK: u16 = 10;

pub fn tileset_tile(tileset: u8, k: u16) -> u16 {
    Spr::CraterTop as u16 + u16::from(tileset) * TILESET_STRIDE + k
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tileset_layout_matches_names() {
        assert_eq!(SPRITE_NAMES[tileset_tile(0, BT_TOP) as usize], "craterTop");
        assert_eq!(
            SPRITE_NAMES[tileset_tile(1, BT_BLOCK) as usize],
            "cavesBlock"
        );
        assert_eq!(
            SPRITE_NAMES[tileset_tile(2, BT_BACK) as usize],
            "citadelBack"
        );
        assert_eq!(
            SPRITE_NAMES[tileset_tile(2, BT_SLOPE0 + 5) as usize],
            "citadelL22B"
        );
        assert_eq!(SPRITE_NAMES[tileset_tile(3, BT_TOP) as usize], "skyTop");
        assert_eq!(SPRITE_NAMES[tileset_tile(3, BT_BACK) as usize], "skyBack");
        assert_eq!(
            SPRITE_NAMES[tileset_tile(4, BT_TOP) as usize],
            "buildingTop"
        );
        assert_eq!(
            SPRITE_NAMES[tileset_tile(4, BT_BACK) as usize],
            "buildingBack"
        );
        assert_eq!(SPRITE_NAMES[tileset_tile(5, BT_TOP) as usize], "theatreTop");
        assert_eq!(
            SPRITE_NAMES[tileset_tile(5, BT_BACK) as usize],
            "theatreBack"
        );
        assert_eq!(SPRITE_NAMES[tileset_tile(6, BT_TOP) as usize], "foundryTop");
        assert_eq!(
            SPRITE_NAMES[tileset_tile(6, BT_BACK) as usize],
            "foundryBack"
        );
        assert_eq!(SPRITE_NAMES[Spr::BenStand as usize], "ben_stand");
        assert_eq!(SPRITE_NAMES[Spr::Star as usize], "star");
        let mut sorted: Vec<_> = SPRITE_NAMES.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), SPRITE_NAMES.len(), "duplicate sprite names");
    }
}
