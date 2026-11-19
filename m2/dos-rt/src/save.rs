// Saving and loading progress as small files (see the C side's `save.c` for where they live).
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// A save is the same progress the web build keeps: lives, score, the next extra life, Fizz, the levels cleared, where Ben stands
// on the map, the time played and the mode (Expert or Normal). The sim takes it back through its state accessors.
//
// The 48 bytes, little-endian:
//   0  "FZSV"        4  version (1)        5  mode: 0 Expert, 1 Normal (saves from before modes have 0 here, which is Expert)
//   6  spare (0)     8  clock stamp       12 lives      16 score      20 next extra life   24 Fizz (ammo)
//   28 levels-cleared mask               32 has a map position   36 map x (f32)   40 map y (f32)   44 seconds played

use core::fmt::Write;
use core::sync::atomic::{AtomicBool, Ordering};

use lf_episode_1::state::{
    AMMO, DONE_MASK, HAS_MAP_POS, LIVES, MAP_X, MAP_Y, NEXT_LIFE, SCORE,
};
use lf_episode_1::{state_get, state_set};

use crate::ui::Buf;

extern "C" {
    fn save_write(slot: i32, data: *const u8, length: i32) -> i32;
    fn save_read(slot: i32, data: *mut u8, length: i32) -> i32;
    fn save_stamp() -> u32;
}

/// The sim's state index of the mode (0 Expert, 1 Normal); it mirrors the main game's `state::NORMAL`.
pub const NORMAL: u32 = 26;
const MAGIC: &[u8; 4] = b"FZSV";
const VERSION: u8 = 1;
const LEN: usize = 48;
/// Slot 0 is the autosave; 1 to 4 are the player's; 5 is the quick-save, which F5 writes and F9 loads.
pub const SLOTS: usize = 6;
/// The quick-save slot.
pub const QUICK: usize = 5;
/// The bits of the cleared mask that are levels (the 16th is the secret).
const LEVEL_BITS: u32 = 0x7FFF;

/// Set for a run that began at the Debug menu's World warp: nothing it does may reach a slot (`write` refuses), so a warp cannot overwrite
/// real progress. The title screen clears it.
static LOCKED: AtomicBool = AtomicBool::new(false);

/// Turns the warp lock on or off.
pub fn set_locked(on: bool) {
    LOCKED.store(on, Ordering::Relaxed);
}

/// Whether saving is off because the run is a warped one.
pub fn locked() -> bool {
    LOCKED.load(Ordering::Relaxed)
}

/// The byte of a save that holds the mode.
const MODE_AT: usize = 5;

/// What a slot's list row shows.
#[derive(Clone, Copy)]
pub struct Info {
    pub levels: u32,
    pub score: i32,
    pub lives: i32,
    pub stamp: u32,
    pub played: u32,
}

fn put(buf: &mut [u8; LEN], at: usize, v: u32) {
    buf[at..at + 4].copy_from_slice(&v.to_le_bytes());
}

fn get(buf: &[u8; LEN], at: usize) -> u32 {
    u32::from_le_bytes([buf[at], buf[at + 1], buf[at + 2], buf[at + 3]])
}

/// The mode a save's bytes hold: Normal only for exactly 1, so an old save (0) and anything unknown are Expert.
fn normal_in(b: &[u8; LEN]) -> bool {
    b[MODE_AT] == 1
}

/// The save's bytes for the sim as it stands now.
fn encode(stamp: u32, played: u32) -> [u8; LEN] {
    let mut b = [0u8; LEN];
    b[..4].copy_from_slice(MAGIC);
    b[4] = VERSION;
    b[MODE_AT] = u8::from(state_get(NORMAL) != 0.0);
    put(&mut b, 8, stamp);
    put(&mut b, 12, state_get(LIVES) as i32 as u32);
    put(&mut b, 16, state_get(SCORE) as i32 as u32);
    put(&mut b, 20, state_get(NEXT_LIFE) as i32 as u32);
    put(&mut b, 24, state_get(AMMO) as i32 as u32);
    put(&mut b, 28, state_get(DONE_MASK) as u32);
    put(&mut b, 32, state_get(HAS_MAP_POS) as u32);
    put(&mut b, 36, (state_get(MAP_X) as f32).to_bits());
    put(&mut b, 40, (state_get(MAP_Y) as f32).to_bits());
    put(&mut b, 44, played);
    b
}

/// Writes the current progress to `slot`; `played` is the seconds played so far. Returns whether it was written (never, while the warp lock is on).
pub fn write(slot: i32, played: u32) -> bool {
    if locked() {
        return false;
    }
    let b = encode(unsafe { save_stamp() }, played);
    unsafe { save_write(slot, b.as_ptr(), LEN as i32) != 0 }
}

fn read(slot: i32) -> Option<[u8; LEN]> {
    let mut b = [0u8; LEN];
    let n = unsafe { save_read(slot, b.as_mut_ptr(), LEN as i32) };
    (n as usize == LEN && &b[..4] == MAGIC && b[4] == VERSION).then_some(b)
}

/// What is saved in `slot`, if anything.
pub fn info(slot: i32) -> Option<Info> {
    read(slot).map(|b| Info {
        levels: (get(&b, 28) & LEVEL_BITS).count_ones(),
        score: get(&b, 16) as i32,
        lives: get(&b, 12) as i32,
        stamp: get(&b, 8),
        played: get(&b, 44),
    })
}

/// Puts a save's progress into the sim (a new game first, so nothing of the last one is left). Returns the seconds played.
pub fn load(slot: i32) -> Option<u32> {
    read(slot).map(|b| apply(&b))
}

/// Puts a save's bytes into the sim. The mode goes first, so everything the sim derives from it afterwards sees it.
fn apply(b: &[u8; LEN]) -> u32 {
    state_set(NORMAL, if normal_in(b) { 1.0 } else { 0.0 });
    state_set(LIVES, f64::from(get(b, 12) as i32));
    state_set(SCORE, f64::from(get(b, 16) as i32));
    state_set(NEXT_LIFE, f64::from(get(b, 20) as i32));
    state_set(AMMO, f64::from(get(b, 24) as i32));
    state_set(DONE_MASK, f64::from(get(b, 28)));
    if get(b, 32) != 0 {
        state_set(MAP_X, f64::from(f32::from_bits(get(b, 36))));
        state_set(MAP_Y, f64::from(f32::from_bits(get(b, 40))));
    } else {
        state_set(HAS_MAP_POS, 0.0);
    }
    get(b, 44)
}

/// The slot with the newest save, the autosave included. The clock ticks in whole seconds, so of two saves with one stamp the lower slot wins: a quick-save is not taken over the map's autosave by chance.
pub fn newest(saves: &[Option<Info>; SLOTS]) -> Option<usize> {
    (0..SLOTS)
        .filter_map(|i| saves[i].map(|s| (i, s.stamp)))
        .max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0)))
        .map(|(i, _)| i)
}

/// A date as `2026-10-15` from seconds since 1970 (UTC; there is no time zone in a DOS file).
pub fn date(stamp: u32, out: &mut Buf) {
    let days = i64::from(stamp / 86_400);
    // Civil from days (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    let _ = write!(out, "{y:04}-{m:02}-{d:02}");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn old_save() -> [u8; LEN] {
        let mut b = [0u8; LEN];
        b[..4].copy_from_slice(MAGIC);
        b[4] = VERSION;
        b
    }

    #[test]
    fn a_save_from_before_modes_is_expert() {
        assert!(!normal_in(&old_save()));
    }

    #[test]
    fn only_a_one_is_normal() {
        let mut b = old_save();
        b[MODE_AT] = 1;
        assert!(normal_in(&b));
        for v in [2, 3, 0x80, 0xFF] {
            b[MODE_AT] = v;
            assert!(!normal_in(&b), "{v} is not a mode");
        }
    }

    #[test]
    fn the_mode_is_byte_five_and_the_layout_is_unchanged() {
        assert_eq!((MODE_AT, LEN, VERSION), (5, 48, 1));
    }

    /// One test for everything that goes through the sim's state (the sim is a global, so two such tests would race). Needs the sim to
    /// know `NORMAL`, which `tools/sync-sim.sh` brings.
    #[test]
    fn a_normal_save_comes_back_normal_and_an_expert_one_expert() {
        state_set(NORMAL, 1.0);
        state_set(LIVES, 3.0);
        let b = encode(77, 120);
        assert_eq!(b[MODE_AT], 1);
        assert_eq!(&b[..5], b"FZSV\x01");
        state_set(NORMAL, 0.0);
        assert_eq!(apply(&b), 120);
        assert_eq!(state_get(NORMAL), 1.0);
        assert_eq!(state_get(LIVES), 3.0);

        state_set(NORMAL, 0.0);
        let b = encode(77, 5);
        assert_eq!(b[MODE_AT], 0);
        state_set(NORMAL, 1.0);
        assert_eq!(apply(&b), 5);
        assert_eq!(state_get(NORMAL), 0.0);
        // An old save loads as Expert over a Normal game.
        state_set(NORMAL, 1.0);
        apply(&old_save());
        assert_eq!(state_get(NORMAL), 0.0);
    }
}
