// Counts the instructions the DOS build's renderer and sim spend on a frame, stage by stage, on a native 32-bit x87 build
// (the closest thing to the DOS code that a modern machine will run), with the CPU's own instruction counter.
//
//   cargo run --release --target i586-unknown-linux-gnu -- [frames]

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
    video::set_night(night);
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
    for t in 0..frames {
        let start = read_counter();
        LAST.store(start, Ordering::Relaxed);
        shell::advance(script(120 + t));
        let count = video::build(1.0);
        sprites += count;
        video::paint(&mut frame, count);
        total += read_counter() - start;
    }
    let sum = frame.iter().fold(0u32, |h, &b| h.wrapping_mul(31).wrapping_add(u32::from(b)));
    println!("{name}: frame checksum {sum:08x}");
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

fn main() {
    let frames: u32 = std::env::args().nth(1).and_then(|a| a.parse().ok()).unwrap_or(300);
    let fd = open_counter();
    if fd < 0 {
        eprintln!("could not open the instruction counter (perf_event_paranoid too strict?)");
        std::process::exit(1);
    }
    FD.store(fd, Ordering::Relaxed);
    video::set_hook(hook);
    scene("map", video::MAP, false, walk, frames);
    scene("level 0 (day)", 0, false, walk, frames);
    scene("level 1 (cave)", 1, false, walk, frames);
    scene("level 1 (cave, night)", 1, true, walk, frames);
    scene("level 9 (theatre)", 9, false, walk, frames);
}
