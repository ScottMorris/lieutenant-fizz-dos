// The sound data the C driver plays, and what Rust needs to know about it: which songs the screens use and how long they are.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// The layout is `m4/export-sound.ts`'s: a header, a song table, an effect table, the caption and level tables, the title, map and
// boss songs and the extra songs. Rust reads the same table the driver does, so a screen can time itself by a song's length even when
// sound is off and the driver never started.

pub static SOUND: &[u8] = include_bytes!("../../../m4/assets/sound.bin");

/// The extra songs, in the order `export-sound.ts` writes them after the ending.
pub const EXTRA_ENDING: usize = 0;
pub const EXTRA_TITLES: usize = 5;

fn rd16(at: usize) -> u32 {
    u32::from(SOUND[at]) | u32::from(SOUND[at + 1]) << 8
}

fn rd32(at: usize) -> u32 {
    rd16(at) | rd16(at + 2) << 16
}

/// Where the tables after the effects begin: the captions, then the levels, then the title, map and boss songs.
fn tail() -> (usize, usize) {
    let (songs, fx, captions, levels) = (SOUND[5] as usize, SOUND[6] as usize, SOUND[7] as usize, SOUND[8] as usize);
    (9 + songs * 8 + fx * 12 + captions + levels, levels)
}

/// The song the title menu plays.
pub fn title_song() -> u8 {
    SOUND[tail().0]
}

/// The song of an extra, or `None` when the data has fewer extras.
pub fn extra_song(n: usize) -> Option<u8> {
    let at = tail().0 + 3;
    if n < SOUND[at] as usize {
        Some(SOUND[at + 1 + n])
    } else {
        None
    }
}

/// How many 560 Hz ticks one pass of a song takes: the waits of its records, which is the loop length, as the driver plays it.
pub fn song_ticks(song: u8) -> u32 {
    let e = 9 + song as usize * 8;
    let (start, len) = (rd32(e) as usize, rd32(e + 4) as usize);
    let end = start + 2 + rd16(start) as usize;
    debug_assert!(len >= 2 && end <= start + len);
    let mut ticks = 0;
    let mut at = start + 2;
    while at + 4 <= end {
        ticks += rd16(at + 2);
        at += 4;
    }
    ticks
}

/// Checks the tables the screens rely on: the titles' song is eight bars at 150 bpm and the title menu's song is the one the data names.
pub fn self_check() -> bool {
    match extra_song(EXTRA_TITLES) {
        Some(s) => song_ticks(s) == 7168 && song_ticks(title_song()) == 34_560,
        None => false,
    }
}
