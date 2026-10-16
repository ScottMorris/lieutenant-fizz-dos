// A starter app that "transitions into the game": a title screen, then the real Episode 1 simulation, shown as
// text. The host trait is the seam a DOS front end (VGA, keyboard IRQ, PIT timer) would fill in later.
//
// usage: fizz-starter [--auto] [--level N] [--seconds S] [--x87-53]
//   --auto      do not wait for Enter (also the default when stdin is not a terminal)

use lf_episode_1::state::{LEVEL_ID, LIVES, PLAYER_X, PLAYER_Y, SCORE};
use lf_episode_1::{
    enter_level, game_new, instance_ptr, out_ptr, render, set_view, sprite_table_ptr, state_get,
    step, table_len,
};
use std::io::{BufRead, IsTerminal, Write};

/// What the game needs from a machine: input as bits, a screen to show a frame on, and time.
trait Host {
    fn wait_for_start(&mut self);
    fn held(&mut self, tick: u32) -> u32;
    fn present(&mut self, frame: &str);
    fn pace(&mut self, tick: u32);
}

/// A Linux terminal: waits for Enter (unless told not to), plays a scripted walk, prints frames.
struct Terminal {
    auto: bool,
    rng: u32,
    hold: u32,
    mask: u32,
}

impl Host for Terminal {
    fn wait_for_start(&mut self) {
        println!("\n  ┌──────────────────────────────────────────┐");
        println!("  │   LIEUTENANT FIZZ — EPISODE 1            │");
        println!("  │   The Cocoa Caper                        │");
        println!("  │   (starter host: sim in Rust, text view) │");
        println!("  └──────────────────────────────────────────┘");
        if !self.auto {
            print!("\n  Press Enter to start... ");
            std::io::stdout().flush().ok();
            let mut line = String::new();
            std::io::stdin().lock().read_line(&mut line).ok();
        }
    }
    fn held(&mut self, _tick: u32) -> u32 {
        if self.hold == 0 {
            self.rng = self.rng.wrapping_mul(1664525).wrapping_add(1013904223);
            let r = self.rng;
            self.mask = (r >> 16) & 0x7f;
            if (r >> 24) & 3 != 0 {
                self.mask = (self.mask | 2) & !1;
            }
            self.hold = 6 + ((r >> 8) & 15);
        }
        self.hold -= 1;
        self.mask
    }
    fn present(&mut self, frame: &str) {
        println!("{frame}");
    }
    fn pace(&mut self, _tick: u32) {}
}

/// The simulation's draw list as text: one character a tile, from the instance buffer the web shell draws.
fn frame_text(w: usize, h: usize) -> String {
    let count = render(1.0, 0) as usize;
    let (cam_x, cam_y) = unsafe { (*out_ptr().add(1), *out_ptr().add(2)) };
    let inst = unsafe { std::slice::from_raw_parts(instance_ptr(), count * 20) };
    let mut grid = vec![b' '; w * h];
    let half_w = w as f32 / 2.0;
    let half_h = h as f32 / 2.0;
    for i in inst.chunks_exact(20) {
        // Matrix row 0 and 1 carry the sprite's size; the translation is its centre in world units.
        let (sw, sh) = (i[0].abs(), i[5].abs());
        let (cx, cy) = (i[12], i[13]);
        let col = (cx - cam_x + half_w).floor();
        let row = (half_h - (cy - cam_y)).floor();
        if col < 0.0 || row < 0.0 || col >= w as f32 || row >= h as f32 {
            continue;
        }
        let ch = if sw >= 0.9 && sh >= 0.9 {
            b'#'
        } else if sw > 0.2 {
            b'o'
        } else {
            b'.'
        };
        grid[row as usize * w + col as usize] = ch;
    }
    // Ben, from the state the HUD reads.
    let (px, py) = (state_get(PLAYER_X) as f32, state_get(PLAYER_Y) as f32);
    let (col, row) = (
        (px - cam_x + half_w).floor(),
        (half_h - (py - cam_y)).floor(),
    );
    if col >= 0.0 && row >= 0.0 && (col as usize) < w && (row as usize) < h {
        grid[row as usize * w + col as usize] = b'@';
    }
    grid.chunks(w)
        .map(|r| format!("  |{}|", String::from_utf8_lossy(r)))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(target_arch = "x86")]
fn fpu_init() {
    // What a DOS start-up does: a 53-bit mantissa, so f64 maths matches the web build bit for bit.
    let cw: u16 = 0x027F;
    unsafe { core::arch::asm!("fldcw [{0}]", in(reg) &cw, options(nostack)) };
}
#[cfg(not(target_arch = "x86"))]
fn fpu_init() {}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let flag = |f: &str| args.iter().any(|a| a == f);
    let value = |f: &str, d: u32| {
        args.iter()
            .position(|a| a == f)
            .and_then(|i| args.get(i + 1))
            .and_then(|v| v.parse().ok())
            .unwrap_or(d)
    };
    if flag("--x87-53") {
        fpu_init();
    }
    let mut host = Terminal {
        auto: flag("--auto") || !std::io::stdin().is_terminal(),
        rng: 0x1234,
        hold: 0,
        mask: 0,
    };
    host.wait_for_start();

    // The transition: the title hands over to the game.
    let level = value("--level", 0);
    game_new();
    enter_level(level);
    set_view(15.0, 7.0);
    // The host owns the sprite sizes (the web shell fills them from its atlas): a tile each, until a real atlas.
    let sprites = table_len(0) as usize;
    let table = unsafe { std::slice::from_raw_parts_mut(sprite_table_ptr(), sprites * 6) };
    for s in table.chunks_exact_mut(6) {
        s[4] = 1.0;
        s[5] = 1.0;
    }
    let seconds = value("--seconds", 6);
    for tick in 1..=seconds * 60 {
        let held = host.held(tick);
        step(held);
        host.pace(tick);
        if tick % 60 == 0 {
            host.present(&format!(
                "  t={}s level={} lives={} score={}",
                tick / 60,
                state_get(LEVEL_ID) as u32,
                state_get(LIVES) as u32,
                state_get(SCORE) as u32
            ));
            if tick == seconds * 60 {
                host.present(&frame_text(30, 14));
            }
        }
    }
}
