// Runs the real Episode 1 simulation through the same C-ABI the web shell uses, with a fixed input script, and prints a hash of
// its state every 60 ticks, so builds can be compared line by line.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// usage: m0 [--x87-53]   (--x87-53 sets the x87 precision control to a 53-bit mantissa first, like an f64)

use lf_episode_1::{enter_level, game_new, state_get, step};

const LEVELS: u32 = 16;
const TICKS: u32 = 3000;
const CHECK_EVERY: u32 = 60;
const STATE_SLOTS: u32 = 26;

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

#[cfg(target_arch = "x86")]
fn set_x87_53() {
    // 0x027F: all exceptions masked, 53-bit precision, round to nearest.
    let cw: u16 = 0x027F;
    unsafe { core::arch::asm!("fldcw [{0}]", in(reg) &cw, options(nostack)) };
}
#[cfg(not(target_arch = "x86"))]
fn set_x87_53() {
    eprintln!("--x87-53 does nothing off 32-bit x86");
}

fn main() {
    if std::env::args().any(|a| a == "--x87-53") {
        set_x87_53();
    }
    for level in 0..LEVELS {
        game_new();
        enter_level(level);
        let mut rng = Lcg(0x1234 + level);
        let mut hold = 0;
        let mut mask = 0u32;
        let mut h = 0xcbf2_9ce4_8422_2325u64;
        for tick in 1..=TICKS {
            if hold == 0 {
                let r = rng.next();
                mask = (r >> 16) & 0x7f;
                if (r >> 24) & 3 != 0 {
                    mask |= 2; // mostly walk right, so the player gets somewhere
                    mask &= !1;
                }
                hold = 6 + ((r >> 8) & 15);
            }
            hold -= 1;
            step(mask);
            if tick % CHECK_EVERY == 0 {
                for i in 0..STATE_SLOTS {
                    fnv(&mut h, &state_get(i).to_bits().to_le_bytes());
                }
                println!("{level} {tick} {h:016x}");
            }
        }
    }
}
