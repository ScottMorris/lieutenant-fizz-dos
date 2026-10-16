// The DOS runtime around the sim: a heap on DJGPP's malloc, a panic handler, and milestone 2's determinism run.
#![no_std]

extern crate alloc;

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

/// One tick of the scripted walk.
#[no_mangle]
pub extern "C" fn fizz_tick(tick: u32) {
    video::advance(video::scripted_input(tick));
}

/// Draws into `frame` (320 * 200 bytes); returns the clear colour for palette slot 17 as 0x00RRGGBB, 6 bits a channel.
#[no_mangle]
pub extern "C" fn fizz_draw(frame: *mut u8) -> u32 {
    let frame = unsafe { core::slice::from_raw_parts_mut(frame, video::W * video::H) };
    video::draw(frame)
}
