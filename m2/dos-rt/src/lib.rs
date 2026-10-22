// The DOS runtime around the sim: a heap on DJGPP's malloc, a panic handler, and milestone 2's determinism run.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![no_std]

extern crate alloc;

pub mod cine;
pub mod fx;
pub mod preview;
pub mod save;
pub mod shell;
pub mod sound_data;
pub mod story;
pub mod ui;
pub mod ui_data;
pub mod video;

use alloc::alloc::{GlobalAlloc, Layout};
use core::ffi::{c_char, c_int, c_void};

extern "C" {
    fn malloc(n: usize) -> *mut c_void;
    fn free(p: *mut c_void);
    fn realloc(p: *mut c_void, n: usize) -> *mut c_void;
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn fflush(f: *mut c_void) -> c_int;
}

struct Heap;

// DJGPP's malloc returns 8-byte aligned blocks, which covers every type the sim uses (f64 at most).
unsafe impl GlobalAlloc for Heap {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        debug_assert!(l.align() <= 8);
        malloc(l.size()).cast()
    }
    unsafe fn dealloc(&self, p: *mut u8, _: Layout) {
        free(p.cast());
    }
    unsafe fn realloc(&self, p: *mut u8, _: Layout, n: usize) -> *mut u8 {
        realloc(p.cast(), n).cast()
    }
}

#[global_allocator]
static HEAP: Heap = Heap;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    unsafe {
        printf(b"PANIC\n\0".as_ptr().cast());
        if let Some(l) = info.location() {
            printf(b"  at line %d\n\0".as_ptr().cast(), l.line() as c_int);
        }
        fflush(core::ptr::null_mut());
    }
    loop {}
}

/// x87 precision control to a 53-bit mantissa, so f64 arithmetic matches the web build bit for bit.
fn fpu_53() {
    let cw: u16 = 0x027F;
    unsafe { core::arch::asm!("fldcw word ptr [{0}]", in(reg) &cw, options(nostack)) };
}

struct Lcg(u32);
impl Lcg {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1664525).wrapping_add(1013904223);
        self.0
    }
}

fn fnv(h: &mut u64, bytes: &[u8]) {
    for b in bytes {
        *h ^= u64::from(*b);
        *h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
}

const LEVELS: u32 = 16;
const STATE_SLOTS: u32 = 26;

/// Milestone 2: the same run as `m0` (16 levels, `ticks` ticks each, a hash every 60), one line a checkpoint.
#[no_mangle]
pub extern "C" fn fizz_m0(ticks: u32, use_53: c_int) {
    use lf_episode_1::{enter_level, game_new, state_get, step};
    if use_53 != 0 {
        fpu_53();
    }
    for level in 0..LEVELS {
        game_new();
        enter_level(level);
        let mut rng = Lcg(0x1234 + level);
        let mut hold = 0;
        let mut mask = 0u32;
        let mut h = 0xcbf2_9ce4_8422_2325u64;
        for tick in 1..=ticks {
            if hold == 0 {
                let r = rng.next();
                mask = (r >> 16) & 0x7f;
                if (r >> 24) & 3 != 0 {
                    mask |= 2;
                    mask &= !1;
                }
                hold = 6 + ((r >> 8) & 15);
            }
            hold -= 1;
            step(mask);
            if tick % 60 == 0 {
                for i in 0..STATE_SLOTS {
                    fnv(&mut h, &state_get(i).to_bits().to_le_bytes());
                }
                unsafe {
                    printf(
                        b"%u %u %08x%08x\n\0".as_ptr().cast(),
                        level,
                        tick,
                        (h >> 32) as u32,
                        h as u32,
                    );
                }
            }
        }
    }
    unsafe { fflush(core::ptr::null_mut()) };
}

// ---------- Milestone 3: the screen ----------

/// Starts `level` and sets up the sprite table.
#[no_mangle]
pub extern "C" fn fizz_start(level: u32, use_53: c_int) {
    if use_53 != 0 {
        fpu_53();
    }
    video::start(level);
}

/// The bits of the scripted walk at `tick`, for runs that must repeat exactly.
#[no_mangle]
pub extern "C" fn fizz_scripted(tick: u32) -> u32 {
    video::scripted_input(tick)
}

/// One tick with the given input bits.
#[no_mangle]
pub extern "C" fn fizz_tick(held: u32) {
    video::advance(held);
}

/// Goes to a level (it wraps round the table), and which level is on now.
#[no_mangle]
pub extern "C" fn fizz_goto(level: u32) {
    video::goto(level);
}

#[no_mangle]
pub extern "C" fn fizz_level() -> u32 {
    video::level()
}

/// Draws into `frame` (320 * 200 bytes), `alpha` of the way from the last tick to the next; returns the clear colour for
/// palette slot 255 as 0x00RRGGBB, 6 bits a channel.
#[no_mangle]
pub extern "C" fn fizz_draw(frame: *mut u8, alpha: f32) -> u32 {
    let frame = unsafe { core::slice::from_raw_parts_mut(frame, video::W * video::H) };
    video::draw(frame, alpha)
}

/// The two halves of `fizz_draw`, so each can be timed: the sim builds its instance list, then it is painted.
#[no_mangle]
pub extern "C" fn fizz_build(alpha: f32) -> u32 {
    video::build(alpha) as u32
}

#[no_mangle]
pub extern "C" fn fizz_paint(frame: *mut u8, count: u32) -> u32 {
    let frame = unsafe { core::slice::from_raw_parts_mut(frame, video::W * video::H) };
    video::paint(frame, count as usize)
}

/// Turns night mode (darker ambient light, the point lights doing the work) on or off.
#[no_mangle]
pub extern "C" fn fizz_night(on: c_int) {
    video::set_night(on != 0);
}

/// Turns low detail (no backdrop layers) on or off.
#[no_mangle]
pub extern "C" fn fizz_detail(low: c_int) {
    video::set_low_detail(low != 0);
}

/// How much health the level's boss has left (0 when there is none), so the music can change for the fight.
#[no_mangle]
pub extern "C" fn fizz_boss_hp() -> u32 {
    video::boss_hp()
}

/// The sound data (songs, effects and the tables that choose them) for the C driver.
#[no_mangle]
pub extern "C" fn fizz_sound_data() -> *const u8 {
    sound_data::SOUND.as_ptr()
}

#[no_mangle]
pub extern "C" fn fizz_sound_len() -> usize {
    sound_data::SOUND.len()
}

/// The sim's mode: 0 none, 1 attract, 2 map, 3 level.
#[no_mangle]
pub extern "C" fn fizz_mode() -> u32 {
    video::sim_mode()
}

/// Back to the overworld.
#[no_mangle]
pub extern "C" fn fizz_goto_map() {
    video::goto_map();
}

/// A check: make the scripted walk press Jump on the map.
#[no_mangle]
pub extern "C" fn fizz_press(on: c_int) {
    video::set_press(on);
}

/// Draws the start-up screen: `values` is `preview::COUNT` numbers from the C side and `cpu` a C string.
#[no_mangle]
pub extern "C" fn fizz_preview(frame: *mut u8, values: *const i32, cpu: *const u8) {
    let frame = unsafe { core::slice::from_raw_parts_mut(frame, video::W * video::H) };
    let v = unsafe { core::slice::from_raw_parts(values, preview::COUNT) };
    let mut name = [0u8; 16];
    let mut n = 0;
    while n < 15 {
        let c = unsafe { *cpu.add(n) };
        if c == 0 {
            break;
        }
        name[n] = c;
        n += 1;
    }
    preview::draw(frame, v, core::str::from_utf8(&name[..n]).unwrap_or("?"));
}

/// Whether the player chose to leave the game from a menu.
#[no_mangle]
pub extern "C" fn fizz_quit() -> u32 {
    u32::from(shell::quit_requested())
}

/// The song a story screen wants (1 is the first of the extra songs), or 0 when the game chooses by itself.
#[no_mangle]
pub extern "C" fn fizz_music_key() -> u32 {
    shell::music_key()
}

/// A check: shows a menu (1 pause, 2 options).
#[no_mangle]
pub extern "C" fn fizz_show(n: u32) {
    shell::show(n);
}
