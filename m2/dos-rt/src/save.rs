// Saving and loading progress as small files (see the C side's `save.c` for where they live).
//
// A save is the same progress the web build keeps: lives, score, the next extra life, Fizz, the levels cleared, where Ben stands
// on the map and the time played. The sim takes it back through its state accessors.

use core::fmt::Write;

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

const MAGIC: &[u8; 4] = b"FZSV";
const VERSION: u8 = 1;
const LEN: usize = 48;
/// Slot 0 is the autosave; 1 to 4 are the player's.
pub const SLOTS: usize = 5;
/// The bits of the cleared mask that are levels (the 16th is the secret).
const LEVEL_BITS: u32 = 0x7FFF;

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

/// Writes the current progress to `slot`; `played` is the seconds played so far. Returns whether it was written.
pub fn write(slot: i32, played: u32) -> bool {
    let mut b = [0u8; LEN];
    b[..4].copy_from_slice(MAGIC);
    b[4] = VERSION;
    put(&mut b, 8, unsafe { save_stamp() });
    put(&mut b, 12, state_get(LIVES) as i32 as u32);
    put(&mut b, 16, state_get(SCORE) as i32 as u32);
    put(&mut b, 20, state_get(NEXT_LIFE) as i32 as u32);
    put(&mut b, 24, state_get(AMMO) as i32 as u32);
    put(&mut b, 28, state_get(DONE_MASK) as u32);
    put(&mut b, 32, state_get(HAS_MAP_POS) as u32);
    put(&mut b, 36, (state_get(MAP_X) as f32).to_bits());
    put(&mut b, 40, (state_get(MAP_Y) as f32).to_bits());
    put(&mut b, 44, played);
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
    let b = read(slot)?;
    state_set(LIVES, f64::from(get(&b, 12) as i32));
    state_set(SCORE, f64::from(get(&b, 16) as i32));
    state_set(NEXT_LIFE, f64::from(get(&b, 20) as i32));
    state_set(AMMO, f64::from(get(&b, 24) as i32));
    state_set(DONE_MASK, f64::from(get(&b, 28)));
    if get(&b, 32) != 0 {
        state_set(MAP_X, f64::from(f32::from_bits(get(&b, 36))));
        state_set(MAP_Y, f64::from(f32::from_bits(get(&b, 40))));
    } else {
        state_set(HAS_MAP_POS, 0.0);
    }
    Some(get(&b, 44))
}

/// The slot with the newest save, the autosave included.
pub fn newest(saves: &[Option<Info>; SLOTS]) -> Option<usize> {
    (0..SLOTS)
        .filter_map(|i| saves[i].map(|s| (i, s.stamp)))
        .max_by_key(|&(_, stamp)| stamp)
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
