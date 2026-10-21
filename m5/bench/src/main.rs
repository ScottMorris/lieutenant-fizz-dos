// Counts the instructions the DOS build's renderer and sim spend on a frame, stage by stage, on a native 32-bit x87 build (the
// closest thing to the DOS code that a modern machine will run), with the CPU's own instruction counter.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//   cargo run --release --target i586-unknown-linux-gnu -- [frames]
//
// Environment: NOCOUNT=1 runs without the instruction counter, SCENE=<word> runs only the scenes whose name holds it, TICKONLY=1
// steps the sim without drawing, DUMP=<dir> writes each scene's last frame, LOW=1 draws at low detail, HIST=1 lists the sprites
// that cover the most pixels, FOOTAGE=<dir> (with CLIP=<name> for one) records clips for the explainer video in video/. Where the
// counter is not allowed (a virtual machine), `m5/bench/profile.sh` counts the instructions under callgrind instead; see
// docs/PERFORMANCE.md.

#![allow(dead_code)]

#[path = "../../../m2/dos-rt/src/preview.rs"]
mod preview;
#[path = "../../../m2/dos-rt/src/save.rs"]
mod save;
#[path = "../../../m2/dos-rt/src/shell.rs"]
mod shell;
#[path = "../../../m2/dos-rt/src/ui.rs"]
mod ui;
#[path = "../../../m2/dos-rt/src/ui_data.rs"]
mod ui_data;
#[path = "../../../m2/dos-rt/src/video.rs"]
mod video;

use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};

// The C side of the DOS build, which this does not have: sound and save files do nothing here.
#[no_mangle]
pub extern "C" fn snd_caption(_: u32) {}
#[no_mangle]
pub extern "C" fn snd_fx(_: i32) {}
#[no_mangle]
pub extern "C" fn snd_set_music(_: i32) {}
#[no_mangle]
pub extern "C" fn snd_set_fx(_: i32) {}
#[no_mangle]
pub extern "C" fn snd_music_enabled() -> i32 {
    1
}
#[no_mangle]
pub extern "C" fn snd_fx_enabled() -> i32 {
    1
}
#[no_mangle]
pub extern "C" fn snd_have_opl() -> i32 {
    1
}
#[no_mangle]
pub extern "C" fn rate_mode() -> i32 {
    0
}

#[no_mangle]
pub extern "C" fn rate_set(_mode: i32) {}

#[no_mangle]
pub extern "C" fn rate_active() -> i32 {
    60
}

#[no_mangle]
pub extern "C" fn detail_mode() -> i32 {
    0
}

#[no_mangle]
pub extern "C" fn detail_set(_mode: i32) {}

#[no_mangle]
pub extern "C" fn detail_active() -> i32 {
    0
}

#[no_mangle]
pub extern "C" fn settings_save(_night: i32) {}

#[no_mangle]
pub extern "C" fn snd_fx_device() -> i32 {
    1
}
#[no_mangle]
pub extern "C" fn snd_set_fx_device(_: i32) {}
#[no_mangle]
pub extern "C" fn save_write(_: i32, _: *const u8, _: i32) -> i32 {
    0
}
#[no_mangle]
pub extern "C" fn save_read(_: i32, _: *mut u8, _: i32) -> i32 {
    0
}
#[no_mangle]
pub extern "C" fn save_stamp() -> u32 {
    0
}

/// A hardware counter of user-space instructions for this thread.
static FD: AtomicI64 = AtomicI64::new(-1);
static LAST: AtomicU64 = AtomicU64::new(0);
static ACC: [AtomicU64; 10] = [const { AtomicU64::new(0) }; 10];

#[repr(C)]
#[derive(Default)]
struct PerfAttr {
    kind: u32,
    size: u32,
    config: u64,
    sample_period: u64,
    sample_type: u64,
    read_format: u64,
    flags: u64,
    wakeup: u32,
    bp_type: u32,
    config1: u64,
    config2: u64,
    branch_sample_type: u64,
    sample_regs_user: u64,
    sample_stack_user: u32,
    clockid: i32,
    sample_regs_intr: u64,
    aux_watermark: u32,
    sample_max_stack: u16,
    reserved: u16,
    pad: [u64; 6],
}

fn open_counter() -> i64 {
    let mut a = PerfAttr::default();
    a.kind = 0; // hardware
    a.size = core::mem::size_of::<PerfAttr>() as u32;
    a.config = 1; // instructions
    a.flags = (1 << 5) | (1 << 6); // exclude the kernel and the hypervisor
    unsafe { libc::syscall(libc::SYS_perf_event_open, &a as *const PerfAttr, 0, -1, -1, 0) as i64 }
}

fn read_counter() -> u64 {
    let mut v = 0u64;
    unsafe { libc::read(FD.load(Ordering::Relaxed) as i32, &mut v as *mut u64 as *mut _, 8) };
    v
}

fn hook(stage: u8) {
    let now = read_counter();
    let last = LAST.swap(now, Ordering::Relaxed);
    ACC[stage as usize].fetch_add(now - last, Ordering::Relaxed);
}

const NAMES: [&str; 10] = ["", "sim render", "", "lights", "clear", "sprites", "shade", "overlay", "sim step", "events"];

fn scene(name: &str, level: u32, night: bool, script: fn(u32) -> u32, frames: u32) {
    // SCENE=<word> runs only the scenes whose name contains it (for a profiler, which wants one scene at a time).
    if std::env::var("SCENE").is_ok_and(|s| !name.contains(s.as_str())) {
        return;
    }
    video::set_night(night);
    video::set_low_detail(std::env::var("LOW").is_ok());
    video::start(level);
    let mut frame = vec![0u8; video::W * video::H];
    // Settle: a few ticks and one frame, so the first frame's set-up is not counted.
    for t in 0..120 {
        shell::advance(script(t));
    }
    let n = video::build(1.0);
    video::paint(&mut frame, n);
    for a in &ACC {
        a.store(0, Ordering::Relaxed);
    }
    let mut sprites = 0;
    let mut total = 0u64;
    // TICKONLY=1 steps the sim and draws nothing, to weigh the sim on its own.
    let tick_only = std::env::var("TICKONLY").is_ok();
    for t in 0..frames {
        let start = read_counter();
        LAST.store(start, Ordering::Relaxed);
        shell::advance(script(120 + t));
        if !tick_only {
            let count = video::build(1.0);
            sprites += count;
            video::paint(&mut frame, count);
        }
        total += read_counter() - start;
    }
    let sum = frame.iter().fold(0u32, |h, &b| h.wrapping_mul(31).wrapping_add(u32::from(b)));
    // DUMP=<dir> writes each scene's last frame there (64000 palette indices), to compare pictures before and after a change.
    if let Ok(dir) = std::env::var("DUMP") {
        let file = format!("{dir}/{}.raw", name.replace(['(', ')', ','], "").replace(' ', "_"));
        std::fs::write(file, &frame).expect("could not write the frame");
    }
    println!("{name}: frame checksum {sum:08x}");
    if std::env::var("HIST").is_ok() {
        let count = video::build(1.0);
        let inst = unsafe { std::slice::from_raw_parts(lf_episode_1::instance_ptr(), count * 20) };
        let mut by: std::collections::HashMap<&str, (usize, f32)> = std::collections::HashMap::new();
        for b in inst.chunks_exact(20) {
            let n = lf_episode_1::sprites::SPRITE_NAMES.get(b[16] as usize).copied().unwrap_or("?");
            let e = by.entry(n).or_insert((0, 0.0));
            e.0 += 1;
            e.1 += (b[0] * 16.0).abs() * (b[5] * 16.0).abs();
        }
        let mut v: Vec<_> = by.into_iter().collect();
        v.sort_by(|a, b| b.1 .1.partial_cmp(&a.1 .1).unwrap());
        for (n, (c, area)) in v.iter().take(14) {
            println!("      {n:<22} {c:>4} sprites {area:>8.0} px");
        }
    }
    println!("{name}: {} sprites a frame, {} instructions a frame", sprites / frames as usize, total / u64::from(frames));
    for (i, a) in ACC.iter().enumerate() {
        let v = a.load(Ordering::Relaxed) / u64::from(frames);
        if v > 0 {
            println!("    {:<11}{:>9}", NAMES[i], v);
        }
    }
}

fn walk(t: u32) -> u32 {
    video::scripted_input(t)
}

/// Pause on the map with Ben standing on a level, choose Resume with Enter and keep it down: the game must not take that as
/// "enter the level". Prints the game's mode (2 map, 3 level) before and after.
fn pause_resume_test() {
    video::start(video::MAP);
    video::set_press(2); // walk to the Crater Fields marker and stand on it, without pressing
    for t in 0..3000 {
        shell::advance(video::scripted_input(t));
    }
    println!("before: mode {} (2 is the map)", lf_episode_1::mode());
    // Control: a fresh press of Enter on the marker enters the level.
    shell::advance(128);
    for _ in 0..40 {
        shell::advance(0);
    }
    println!("control, a fresh Enter on the marker: mode {} (3 is a level)", lf_episode_1::mode());

    // The bug: pause, Resume with Enter, keep Enter down for a while.
    video::start(video::MAP);
    video::set_press(2);
    for t in 0..3000 {
        shell::advance(video::scripted_input(t));
    }
    shell::advance(256); // Esc: the pause menu
    shell::advance(0);
    shell::advance(128); // Enter on Resume
    for _ in 0..40 {
        shell::advance(128); // still held
    }
    println!("pause, Resume with Enter held: mode {} (2 is right; 3 is the bug)", lf_episode_1::mode());
}

/// Writes one frame of footage: the 64,000 palette indices and the level's clear colour (palette slot 255) as three 6-bit
/// channels.
fn write_frame(dir: &std::path::Path, n: u32, frame: &[u8], clear: u32) {
    let mut bytes = Vec::with_capacity(frame.len() + 3);
    bytes.extend_from_slice(frame);
    bytes.extend_from_slice(&[(clear >> 16) as u8 & 63, (clear >> 8) as u8 & 63, clear as u8 & 63]);
    std::fs::write(dir.join(format!("{n:05}.raw")), bytes).expect("could not write a frame");
}

/// A walk to the right with jumps every `period` ticks held for `hold`, walking `duty` percent of the time.
#[derive(Clone, Copy)]
struct Walk {
    period: u32,
    hold: u32,
    phase: u32,
    duty: u32,
}

impl Walk {
    fn input(self, t: u32) -> u32 {
        const RIGHT: u32 = 2;
        const JUMP: u32 = 16;
        let walk = if t % 100 < self.duty { RIGHT } else { 0 };
        walk | if (t + self.phase) % self.period < self.hold { JUMP } else { 0 }
    }
}

/// The walk, from a small family, that keeps Ben alive longest in `level` (and then gets furthest), so the footage shows play
/// rather than the card after a fall. Only the sim runs here, which is quick.
fn surviving_walk(level: u32, night: bool, ticks: u32) -> Walk {
    let mut best = (0u32, f64::MIN, Walk { period: 45, hold: 14, phase: 0, duty: 100 });
    for period in (20..=90).step_by(5) {
        for hold in [4, 8, 12, 16, 20] {
            if hold >= period {
                continue;
            }
            for phase in [0, 10, 20, 30] {
                for duty in [100, 85, 70] {
                    let w = Walk { period, hold, phase, duty };
                    video::set_night(night);
                    video::start(level);
                    let x0 = lf_episode_1::state_get(lf_episode_1::state::PLAYER_X);
                    let mut alive = 0;
                    for t in 0..ticks {
                        shell::advance(w.input(t));
                        if lf_episode_1::state_get(lf_episode_1::state::PLAYER_DEAD) != 0.0 || lf_episode_1::mode() != 3 {
                            break;
                        }
                        alive = t + 1;
                    }
                    let gone = lf_episode_1::state_get(lf_episode_1::state::PLAYER_X) - x0;
                    if (alive, gone) > (best.0, best.1) {
                        best = (alive, gone, w);
                    }
                }
            }
        }
    }
    best.2
}

/// FOOTAGE=<dir>: records clips of the real renderer and shell for the explainer video (video/), one tick a frame, keeping every
/// second frame (30 a second). The day level and the map use the scripted walk; the caves and level 9 use the walk that survives
/// longest there. Each clip is a folder of `NNNNN.raw` frames; video/ adds the palette.
fn footage(dir: &str) {
    let walk = |t: u32| video::scripted_input(t);
    let still = |_: u32| 0;
    let caves = surviving_walk(1, false, 900);
    let theatre = surviving_walk(9, false, 720);
    let caves_walk = move |t: u32| caves.input(t);
    let theatre_walk = move |t: u32| theatre.input(t);
    // Walk a while, then Esc for the pause menu and stay on it.
    let pause = |t: u32| if t < 200 { video::scripted_input(t) } else if t == 200 { 256 } else { 0 };
    #[allow(clippy::type_complexity)]
    let clips: [(&str, u32, bool, u32, u32, &dyn Fn(u32) -> u32); 7] = [
        ("title", video::TITLE, false, 0, 600, &still),
        ("map", video::MAP, false, 60, 960, &walk),
        ("day", 0, false, 30, 900, &walk),
        ("caves", 1, false, 30, 900, &caves_walk),
        ("night", 1, true, 30, 600, &caves_walk),
        ("theatre", 9, false, 30, 720, &theatre_walk),
        ("pause", 0, false, 150, 400, &pause),
    ];
    let only = std::env::var("CLIP").ok();
    let mut frame = vec![0u8; video::W * video::H];
    for (name, level, night, from, to, input) in clips {
        if only.as_deref().is_some_and(|c| c != name) {
            continue;
        }
        let out = std::path::Path::new(dir).join(name);
        std::fs::create_dir_all(&out).expect("could not make the clip's folder");
        video::set_night(night);
        video::set_low_detail(false);
        video::start(level);
        let mut n = 0;
        for t in 0..to {
            shell::advance(input(t));
            if t >= from && (t - from) % 2 == 0 {
                let count = video::build(1.0);
                let clear = video::paint(&mut frame, count);
                write_frame(&out, n, &frame, clear);
                n += 1;
            }
        }
        println!("{name}: {n} frames");
    }
    // The start-up system check, as a 486DX2 with an AdLib card would show it (the values are typical, not measured).
    if only.as_deref().is_none_or(|c| c == "preview") {
        let out = std::path::Path::new(dir).join("preview");
        std::fs::create_dir_all(&out).expect("could not make the clip's folder");
        let mut v = [0i32; preview::COUNT];
        v[preview::VGA] = 1;
        v[preview::EGA] = 1;
        v[preview::ADLIB] = 1;
        v[preview::BLASTER] = 1;
        v[preview::FPU] = 1;
        v[preview::MEMORY_KB] = 16384;
        v[preview::FPS] = 23;
        v[preview::SOUND_CHOICE] = 0;
        v[preview::SOUND_USING] = 1;
        preview::draw(&mut frame, &v, "486DX2");
        write_frame(&out, 0, &frame, 0);
        println!("preview: 1 frame");
    }
}

fn main() {
    if let Ok(dir) = std::env::var("FOOTAGE") {
        footage(&dir);
        return;
    }
    if std::env::var("PAUSETEST").is_ok() {
        pause_resume_test();
        return;
    }
    let frames: u32 = std::env::args().nth(1).and_then(|a| a.parse().ok()).unwrap_or(300);
    // NOCOUNT=1 skips the instruction counter (a CI runner may not allow it) and only prints the frame checksums.
    if std::env::var("NOCOUNT").is_err() {
        let fd = open_counter();
        if fd < 0 {
            eprintln!("could not open the instruction counter (perf_event_paranoid too strict?); NOCOUNT=1 runs without it");
            std::process::exit(1);
        }
        FD.store(fd, Ordering::Relaxed);
        video::set_hook(hook);
    }
    scene("map", video::MAP, false, walk, frames);
    scene("level 0 (day)", 0, false, walk, frames);
    scene("level 1 (cave)", 1, false, walk, frames);
    scene("level 1 (cave, night)", 1, true, walk, frames);
    scene("level 9 (theatre)", 9, false, walk, frames);
}
