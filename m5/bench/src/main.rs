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
// that cover the most pixels, TITLES_DUMP=<file> writes every tick of one pass of the opening titles (768 frames) for
// `bun m4/check-titles.ts --dump <file>`. STARTUPTEST=1 prints the text start-up screen's cells for a set of machines, which
// tools/check.sh compares with m5/bench/expected-startup-cells.txt; STARTUPVALUES=<the eight numbers>,<processor> (as `FIZZ
// previewtext` prints them) writes that screen's 4000 bytes, as CELLS.RAW holds them, to standard output. Where the counter is not allowed (a virtual machine), `valgrind --tool=callgrind` counts the
// instructions instead; see docs/PERFORMANCE.md.

#![allow(dead_code)]

#[path = "../../../m2/dos-rt/src/cine.rs"]
mod cine;
#[path = "../../../m2/dos-rt/src/debug.rs"]
mod debug;
#[path = "../../../m2/dos-rt/src/fx.rs"]
mod fx;
#[path = "../../../m2/dos-rt/src/preview.rs"]
mod preview;
#[path = "../../../m2/dos-rt/src/save.rs"]
mod save;
#[path = "../../../m2/dos-rt/src/shell.rs"]
mod shell;
#[path = "../../../m2/dos-rt/src/sound_data.rs"]
mod sound_data;
#[path = "../../../m2/dos-rt/src/story.rs"]
mod story;
#[path = "../../../m2/dos-rt/src/titles.rs"]
mod titles;
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
pub extern "C" fn snd_current_song() -> i32 {
    -1
}
#[no_mangle]
pub extern "C" fn snd_music_clock() -> i32 {
    -1
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
pub extern "C" fn scheme_mode() -> i32 {
    1
}

#[no_mangle]
pub extern "C" fn scheme_set(_scheme: i32) {}

#[no_mangle]
pub extern "C" fn keymap_step(scheme: i32, _dir: i32) -> i32 {
    scheme
}

#[no_mangle]
pub extern "C" fn mouse_present() -> i32 {
    0
}

#[no_mangle]
pub extern "C" fn snd_fx_device() -> i32 {
    1
}
#[no_mangle]
pub extern "C" fn snd_set_fx_device(_: i32) {}
#[no_mangle]
pub extern "C" fn save_write(_: i32, _: *const u8, _: i32) -> i32 {
    SAVE_WRITES.fetch_add(1, Ordering::Relaxed);
    0
}
/// How many times the shell asked for a save file to be written.
static SAVE_WRITES: AtomicU64 = AtomicU64::new(0);
/// The raw key presses a test has queued for the shell, oldest first (what the C side's `kbd_raw_pop` hands over).
static RAW_KEYS: std::sync::Mutex<std::collections::VecDeque<u8>> = std::sync::Mutex::new(std::collections::VecDeque::new());
#[no_mangle]
pub extern "C" fn kbd_raw_pop() -> i32 {
    RAW_KEYS.lock().unwrap().pop_front().map_or(-1, i32::from)
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
    scene_on(name, level, 0, night, script, frames);
}

/// `scene`, with a screen put up after the start (`shell::show`'s number; 0 for none).
fn scene_on(name: &str, level: u32, show: u32, night: bool, script: fn(u32) -> u32, frames: u32) {
    // SCENE=<word> runs only the scenes whose name contains it (for a profiler, which wants one scene at a time).
    if std::env::var("SCENE").is_ok_and(|s| !name.contains(s.as_str())) {
        return;
    }
    video::set_night(night);
    video::set_low_detail(std::env::var("LOW").is_ok());
    video::start(level);
    if show != 0 {
        shell::show(show);
    }
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

fn no_keys(_: u32) -> u32 {
    0
}

/// Every tick of one pass of the opening titles, through the shell and the renderer as the game draws them: tick n is the frame after
/// n ticks, the first drawn before any.
fn titles_dump(file: &str) {
    video::start(video::TITLE);
    shell::show(200);
    let mut frame = vec![0u8; video::W * video::H];
    let mut all = Vec::with_capacity(frame.len() * titles::LOOP as usize);
    for n in 0..titles::LOOP {
        if n > 0 {
            shell::advance(0);
        }
        let count = video::build(1.0);
        video::paint(&mut frame, count);
        all.extend_from_slice(&frame);
    }
    std::fs::write(file, &all).expect("could not write the titles' frames");
    println!("wrote {} ticks of the titles to {file}", titles::LOOP);
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

/// One scenario of `flicker_test`: draws `frames` frames, with a `shell::advance(input)` before every third (a machine quicker than the
/// tick) and the interpolation factor the main loop would give it (it climbs between ticks), and reports how many pixels differ from the
/// frame before within a tick. Between ticks the picture may change only by the sim's own doing (the soda can bobbing, water), which a
/// tick brings and `alpha` does not.
fn flicker_run(name: &str, frames: u32, input: u32, bad: &mut u32) {
    let mut prev = vec![0u8; video::W * video::H];
    let mut frame = vec![0u8; video::W * video::H];
    let mut changed = 0usize;
    let mut worst = 0usize;
    let mut bbox = (i32::MAX, i32::MAX, -1, -1);
    for k in 0..frames {
        if k % 3 == 0 {
            shell::advance(input);
        }
        let alpha = (k % 3) as f32 / 3.0 + 0.1;
        let n = video::build(alpha);
        video::paint(&mut frame, n);
        if k % 3 != 0 {
            let mut here = 0;
            for (i, (a, b)) in frame.iter().zip(prev.iter()).enumerate() {
                if a != b {
                    here += 1;
                    let (x, y) = ((i % video::W) as i32, (i / video::W) as i32);
                    bbox = (bbox.0.min(x), bbox.1.min(y), bbox.2.max(x), bbox.3.max(y));
                }
            }
            if here > 0 {
                changed += 1;
            }
            worst = worst.max(here);
        }
        prev.copy_from_slice(&frame);
    }
    if changed == 0 {
        println!("flicker: {name}: still");
    } else {
        *bad += 1;
        println!("flicker: {name}: {changed} of {} frames differ from the one before in their tick, up to {worst} pixels, within x {}..{} y {}..{}", frames - frames.div_ceil(3), bbox.0, bbox.2, bbox.1, bbox.3);
    }
}

/// A scene that stands still in the game's terms must draw the same picture every frame, however the interpolation factor moves: the
/// pause menu and the screens that open from it, a level with Ben at rest, and the map's prompt. Prints what differs.
fn flicker_test() {
    let mut bad = 0;
    const ESC: u32 = 256;
    // A level with Ben at rest, no menu.
    video::start(0);
    for t in 0..200 {
        shell::advance(walk(t));
    }
    for _ in 0..120 {
        shell::advance(0);
    }
    // The sim runs here, so a patrolling enemy or a moving platform glides between ticks on purpose: reported, not counted.
    flicker_run("level, Ben at rest (moving things glide)", 24, 0, &mut 0);
    // The pause menu opened while Ben was running, and again after he had stopped.
    video::start(0);
    for t in 0..200 {
        shell::advance(walk(t));
    }
    shell::advance(ESC);
    shell::advance(0);
    flicker_run("pause, opened while running", 24, 0, &mut bad);
    video::start(0);
    for t in 0..200 {
        shell::advance(walk(t));
    }
    for _ in 0..120 {
        shell::advance(0);
    }
    shell::advance(ESC);
    shell::advance(0);
    flicker_run("pause, opened with Ben at rest", 24, 0, &mut bad);
    for (name, n) in [("options", 2), ("controls", 6), ("load list", 3), ("end card", 9)] {
        video::start(0);
        for t in 0..200 {
            shell::advance(walk(t));
        }
        shell::show(n);
        shell::advance(0);
        flicker_run(&format!("{name} over a level"), 24, 0, &mut bad);
    }
    // The map's prompt, with Ben standing on a marker.
    video::start(video::MAP);
    video::set_press(2);
    for t in 0..3000 {
        shell::advance(video::scripted_input(t));
    }
    flicker_run("map, Ben on a marker", 24, 0, &mut bad);
    if bad == 0 {
        println!("flicker ok");
    } else {
        std::process::exit(1);
    }
}

/// The titles' flow: the opening at the start, a key skipping it, and the long wait on the title menu bringing it back.
fn flow_test() {
    let mut bad = 0;
    let mut check = |what: &str, ok: bool| {
        println!("flow: {what}: {}", if ok { "yes" } else { "NO" });
        if !ok {
            bad += 1;
        }
    };
    const ANY: u32 = 2048;
    let run = |n: u32| {
        for _ in 0..n {
            shell::advance(0);
        }
    };

    // The start: the titles play out their one loop (768 ticks) and the menu follows.
    video::start(video::TITLE);
    shell::boot_titles();
    run(767);
    check("the titles are up for their whole loop", shell::on_titles());
    run(1);
    check("the menu follows the loop", shell::on_title());

    // A key skips: a fade of 12 ticks, then the menu.
    video::start(video::TITLE);
    shell::boot_titles();
    run(100);
    shell::advance(ANY);
    run(11);
    check("a key's fade is still running 11 ticks on", shell::on_titles());
    run(2);
    check("the menu is up 13 ticks after a key", shell::on_title());

    // A key held from before the titles began does not skip them: only a new press does.
    video::start(video::TITLE);
    shell::boot_titles();
    shell::advance(0);
    run(30);
    check("the titles run while nothing is pressed", shell::on_titles());

    // The wait: two loops of the title's song, 7394 ticks of the menu without music (the tick that ends its fade-in does not count), then a fade and the titles for two loops.
    video::start(video::TITLE);
    run(7392);
    check("the menu waits 7392 ticks", shell::on_title());
    run(1 + 14);
    check("the titles are back once the wait and the fade are over", shell::on_titles());
    run(1535);
    check("the titles play their two loops", shell::on_titles());
    run(1);
    check("the menu follows the second loop", shell::on_title());

    // A key on the menu starts the wait again.
    video::start(video::TITLE);
    run(7000);
    shell::advance(ANY);
    run(7000);
    check("a key 7000 ticks in sets the wait back", shell::on_title());
    run(1000);
    check("and the titles come 7394 ticks after it", shell::on_titles());

    // Night: the Options row and the N key share one flag, so N after Options turned night on turns it off.
    const RIGHT: u32 = 2;
    const DOWN: u32 = 8;
    video::start(video::TITLE);
    video::set_night(false);
    shell::show(2);
    for _ in 0..5 {
        shell::advance(DOWN); // Music, Effects, Effects play on, Frame rate, Detail, then Night
        shell::advance(0);
    }
    shell::advance(RIGHT);
    shell::advance(0);
    check("the Options row turns night on", video::night());
    video::toggle_night();
    check("N then turns it off", !video::night());
    video::toggle_night();
    check("and N again back on", video::night());
    video::set_night(false);

    if bad == 0 {
        println!("flow ok");
    } else {
        std::process::exit(1);
    }
}

/// The hidden Debug menu: the key sequence from every kind of screen, a lone key doing nothing, the swallowed Enter, the held game, Back, and a
/// warp that writes no save and ends at the title.
fn debug_test() {
    let mut bad = 0;
    let mut check = |what: &str, ok: bool| {
        println!("debug: {what}: {}", if ok { "yes" } else { "NO" });
        if !ok {
            bad += 1;
        }
    };
    const ENTER: u32 = 128;
    const ESC: u32 = 256;
    const DOWN: u32 = 8;
    const RIGHT: u32 = 2;
    // One tick with `held`, after queuing the keys that came with it.
    let tick = |held: u32, keys: &[u8]| {
        for &k in keys {
            RAW_KEYS.lock().unwrap().push_back(k);
        }
        shell::advance(held);
    };
    let run = |n: u32| {
        for _ in 0..n {
            shell::advance(0);
        }
    };
    let (two, enter) = (debug::SC_2, debug::SC_ENTER);

    // On the title: a lone 2, a lone Enter and Enter then 2 do not open it.
    video::start(video::TITLE);
    run(30);
    tick(0, &[two]);
    run(5);
    check("a lone 2 does not open it", shell::on_title());
    run(200);
    tick(ENTER, &[enter]);
    run(1);
    check("a lone Enter does not open it", !shell::debug_open());

    // The sequence on the title. The Enter that completes it chooses nothing underneath: the menu does not start a game.
    video::start(video::TITLE);
    run(30);
    tick(0, &[two]);
    run(3);
    tick(ENTER, &[enter]);
    check("2 then Enter opens the menu on the title", shell::debug_open());
    // Enter is still down for a few ticks: it chooses nothing on the Debug menu either.
    for _ in 0..10 {
        shell::advance(ENTER);
    }
    check("the Enter that completed it did not choose a Debug row", shell::debug_open() && !shell::debug_warp_page());
    run(30);
    check("and none of its press chose New game underneath", shell::debug_open());
    shell::advance(ESC);
    shell::advance(0);
    check("Esc puts the title back", shell::on_title());

    // Too slow: 2, then Enter 1.6 seconds later (96 ticks), does nothing.
    video::start(video::TITLE);
    run(30);
    tick(0, &[two]);
    run(95);
    tick(0, &[enter]);
    check("Enter 1.6 s after 2 does not open it", !shell::debug_open());
    video::start(video::TITLE);
    run(30);
    tick(0, &[two]);
    run(85);
    tick(0, &[enter]);
    check("Enter 1.4 s after 2 opens it", shell::debug_open());

    // A wrong key between them drops the progress.
    video::start(video::TITLE);
    run(30);
    tick(0, &[two]);
    run(3);
    tick(0, &[0x1E]);
    run(3);
    tick(0, &[enter]);
    check("a key between 2 and Enter drops the sequence", !shell::debug_open());

    // Over the opening titles it is ignored.
    video::start(video::TITLE);
    shell::boot_titles();
    run(5);
    tick(0, &[two]);
    run(2);
    tick(0, &[enter]);
    check("the opening titles ignore it", shell::on_titles() && !shell::debug_open());

    // In a level: the game is held while it is up, and Back puts it back.
    video::start(0);
    run(120);
    let before = shell::play_ticks();
    tick(0, &[two]);
    run(2);
    tick(ENTER, &[enter]);
    check("it opens in a level", shell::debug_open());
    let held_at = shell::play_ticks();
    run(120);
    check("the level is held while the menu is up", shell::play_ticks() == held_at && held_at > before);
    shell::advance(ESC);
    shell::advance(0);
    check("Esc goes back to the level", !shell::debug_open() && shell::on_play());
    run(10);
    check("and the level runs again", shell::play_ticks() > held_at);

    // On the map and over the pause menu.
    video::start(video::MAP);
    run(30);
    tick(0, &[two]);
    tick(ENTER, &[enter]);
    check("it opens on the map", shell::debug_open());
    shell::advance(ESC);
    shell::advance(0);
    shell::show(1);
    run(5);
    tick(0, &[two]);
    tick(ENTER, &[enter]);
    check("it opens over the pause menu", shell::debug_open());
    shell::advance(ESC);
    shell::advance(0);
    check("and Back returns to the pause menu", shell::on_pause());

    // The opening cinematic, the ending and the credits cannot be held: Back lands on the title.
    for (name, n) in [("cinematic", 100), ("ending", 7), ("credits", 8)] {
        video::start(0);
        shell::show(n);
        run(5);
        tick(0, &[two]);
        tick(ENTER, &[enter]);
        check(&format!("it opens over the {name}"), shell::debug_open());
        shell::advance(ESC);
        shell::advance(0);
        check(&format!("and Back after the {name} lands on the title"), shell::on_title());
    }

    // The Mode row sits between Warp to level and Back and flips the mode; Left and Right flip it too.
    video::start(video::TITLE);
    run(30);
    check("a game starts in Expert", !shell::normal_chosen());
    shell::show(110);
    shell::advance(DOWN);
    shell::advance(0);
    shell::advance(ENTER);
    shell::advance(0);
    check("Enter on the Mode row makes it Normal", shell::normal_chosen() && shell::debug_open() && !shell::debug_warp_page());
    shell::advance(ENTER);
    shell::advance(0);
    check("and again makes it Expert", !shell::normal_chosen() && shell::debug_open());
    shell::advance(RIGHT);
    shell::advance(0);
    check("Right on the row flips it", shell::normal_chosen());
    shell::advance(DOWN);
    shell::advance(0);
    shell::advance(ENTER);
    shell::advance(0);
    check("Back is the row after Mode, and keeps the mode", !shell::debug_open() && shell::normal_chosen());

    // A warp: the list, a pick, the level, no save written, and the title at the end.
    video::start(video::TITLE);
    run(30);
    let writes = SAVE_WRITES.load(Ordering::Relaxed);
    tick(0, &[two]);
    tick(ENTER, &[enter]);
    run(2);
    tick(ENTER, &[]);
    run(1);
    check("Enter on Warp to level opens the list", shell::debug_warp_page());
    for _ in 0..4 {
        shell::advance(DOWN);
        shell::advance(0);
    }
    shell::advance(ENTER);
    shell::advance(0);
    run(40);
    check("the pick starts that level (the fifth row is level 4, Zarg Lookout)", shell::on_play() && video::level() == 4 && lf_episode_1::mode() == 3);
    check("the run is a warped one", save::locked());
    check("the warp keeps the chosen mode", shell::normal_chosen());
    // F5 (512) and the pause menu's slots must not write.
    shell::advance(512);
    shell::advance(0);
    check("F5 writes nothing in a warped run", SAVE_WRITES.load(Ordering::Relaxed) == writes);
    shell::show(4);
    shell::advance(ENTER);
    shell::advance(0);
    check("Save game's slot writes nothing in a warped run", SAVE_WRITES.load(Ordering::Relaxed) == writes && save::locked());
    check("the save call itself refuses", !save::write(1, 0));
    shell::advance(ESC);
    shell::advance(0);
    // Quit to title from a level, then a warp again and the level's end.
    shell::show(1);
    shell::advance(0);
    // A warped run has no "Back to the map" row, so Quit to title is the fifth and last.
    for _ in 0..4 {
        shell::advance(DOWN);
        shell::advance(0);
    }
    shell::advance(ENTER);
    shell::advance(0);
    run(40);
    check("Quit to title ends the warp", shell::on_title() && !save::locked());
    check("and nothing was written on the way", SAVE_WRITES.load(Ordering::Relaxed) == writes);

    // A warped level's end goes to the title, with no autosave.
    video::start(video::TITLE);
    run(30);
    tick(0, &[two]);
    tick(ENTER, &[enter]);
    run(2);
    tick(ENTER, &[]);
    run(1);
    shell::advance(ENTER);
    shell::advance(0);
    run(40);
    check("a warp to the first level", shell::on_play() && video::level() == 0 && save::locked());
    // Ben falls out of the world: the card for a lost life comes up, and Enter on it would go back to the map.
    lf_episode_1::state_set(lf_episode_1::state::PLAYER_Y, -50.0);
    run(300);
    check("falling out of the level brings the card", shell::card_up());
    run(30);
    shell::advance(ENTER);
    shell::advance(0);
    run(60);
    check("a warped level that ends goes to the title, not the map", shell::on_title() && !save::locked());
    check("with no autosave written", SAVE_WRITES.load(Ordering::Relaxed) == writes);

    // An ordinary run still autosaves on the map.
    video::start(video::TITLE);
    run(30);
    shell::show(10);
    run(20);
    check("a load is not a warp, and its map visit autosaves", !save::locked());

    // The New game screen: Normal is under the cursor, Esc goes back to the title, and the pick starts the opening in that mode.
    video::start(video::TITLE);
    run(30);
    shell::advance(ENTER);
    shell::advance(0);
    check("New game opens the mode screen", shell::mode_screen_up());
    shell::advance(ESC);
    shell::advance(0);
    check("Esc on it goes back to the title", shell::on_title() && !shell::mode_screen_up());
    shell::advance(ENTER);
    shell::advance(0);
    shell::advance(DOWN);
    shell::advance(0);
    shell::advance(ENTER);
    shell::advance(0);
    run(40);
    check("Expert is the second row, and starts the opening in Expert", !shell::mode_screen_up() && !shell::on_title() && !shell::normal_chosen());
    video::start(video::TITLE);
    run(30);
    shell::advance(ENTER);
    shell::advance(0);
    shell::advance(ENTER);
    shell::advance(0);
    run(40);
    check("Normal is the first row, and starts the opening in Normal", !shell::mode_screen_up() && !shell::on_title() && shell::normal_chosen());

    if bad == 0 {
        println!("debug ok");
    } else {
        std::process::exit(1);
    }
}

/// The text start-up screen for a set of machines: for each, its name, the 25 rows of characters (code page 437 shown as
/// Unicode) and the 25 rows of attributes as two hex digits a cell.
fn startup_cells() {
    use preview::*;
    // name, video, mouse, joystick, AdLib, Sound Blaster, x87, free KB, frames a second, processor
    let machines: [(&str, i32, i32, i32, i32, i32, i32, i32, i32, &str); 12] = [
        ("i486, colour VGA, AdLib, mouse, 13 MB, 71 fps (fine)", VIDEO_VGA, 1, 0, 1, 0, 1, 13 * 1024, 71, "i486"),
        ("i386, no x87, mono VGA, mouse, 4 MB, 25 fps", VIDEO_VGA_MONO, 1, 0, 0, 0, 0, 4 * 1024, 25, "i386"),
        ("EGA, i486, AdLib, mouse, 8 MB", VIDEO_EGA, 1, 0, 1, 0, 1, 8 * 1024, 45, "i486"),
        ("CGA, i486, AdLib, mouse, 8 MB", VIDEO_CGA, 1, 0, 1, 0, 1, 8 * 1024, 45, "i486"),
        ("Hercules or MDA, i486, AdLib, mouse, 8 MB", VIDEO_MDA, 1, 0, 1, 0, 1, 8 * 1024, 45, "i486"),
        ("Under 1 MB free, colour VGA, i486", VIDEO_VGA, 1, 0, 1, 0, 1, 512, 70, "i486"),
        ("No VGA, no x87, under 1 MB", VIDEO_EGA, 0, 0, 0, 0, 0, 512, 0, "i386"),
        ("i586 with everything, 16 MB, 95 fps (smooth)", VIDEO_VGA, 1, 1, 1, 1, 1, 16 * 1024, 95, "i586"),
        ("i486, colour VGA, 16 MB, 45 fps (borderline)", VIDEO_VGA, 1, 0, 1, 0, 1, 16 * 1024, 45, "i486"),
        ("i486, colour VGA, 16 MB, 25 fps (too slow)", VIDEO_VGA, 1, 0, 1, 0, 1, 16 * 1024, 25, "i486"),
        ("i486, colour VGA, 4 MB, 71 fps (warning, can start)", VIDEO_VGA, 1, 0, 1, 0, 1, 4 * 1024, 71, "i486"),
        ("i686, colour VGA, 13 MB, speed test running", VIDEO_VGA, 1, 0, 1, 0, 1, 13 * 1024, FPS_RUNNING, "i686"),
    ];
    let cp437 = |b: u8| -> char {
        match b {
            0x1E => '\u{25B2}',
            0xB0 => '\u{2591}',
            0xB2 => '\u{2593}',
            0xBA => '\u{2551}',
            0xBB => '\u{2557}',
            0xBC => '\u{255D}',
            0xC8 => '\u{255A}',
            0xC9 => '\u{2554}',
            0xCD => '\u{2550}',
            0xFB => '\u{221A}',
            0x20..=0x7E => b as char,
            _ => '?',
        }
    };
    for (name, video, mouse, joy, adlib, blaster, fpu, kb, fps, cpu) in machines {
        let mut v = [0i32; COUNT];
        v[VIDEO] = video;
        v[MOUSE] = mouse;
        v[JOYSTICK] = joy;
        v[ADLIB] = adlib;
        v[BLASTER] = blaster;
        v[FPU] = fpu;
        v[MEMORY_KB] = kb;
        v[FPS] = fps;
        let mut out = [0u16; CELLS];
        cells(&mut out, &v, cpu);
        println!("== {name}");
        for row in out.chunks(COLS) {
            println!("{}", row.iter().map(|&c| cp437(c as u8)).collect::<String>());
        }
        for row in out.chunks(COLS) {
            println!("{}", row.iter().map(|&c| format!("{:02x}", c >> 8)).collect::<String>());
        }
    }
}

fn main() {
    // The opening cinematic's sprite names must all be in the sheet (a name that drifts from the export would draw nothing).
    assert_eq!(cine::missing_sprites(), 0, "a sprite of the cinematic is not in the sheet");
    assert!(sound_data::self_check(), "the sound data lacks the titles' song or a song is not the length the pictures expect");
    assert!(fx::self_check(), "an easing or a tween is off");
    if std::env::var("PAUSETEST").is_ok() {
        pause_resume_test();
        return;
    }
    if std::env::var("DEBUGTEST").is_ok() {
        debug_test();
        return;
    }
    if std::env::var("PAUSEFLICKER").is_ok() {
        flicker_test();
        return;
    }
    if std::env::var("FLOWTEST").is_ok() {
        flow_test();
        return;
    }
    if let Ok(list) = std::env::var("STARTUPVALUES") {
        let parts: Vec<&str> = list.split(',').collect();
        let mut v = [0i32; preview::COUNT];
        for (slot, part) in v.iter_mut().zip(&parts) {
            *slot = part.trim().parse().expect("a number");
        }
        let mut out = [0u16; preview::CELLS];
        preview::cells(&mut out, &v, parts.get(preview::COUNT).copied().unwrap_or("x86"));
        let bytes: Vec<u8> = out.iter().flat_map(|c| c.to_le_bytes()).collect();
        std::io::Write::write_all(&mut std::io::stdout(), &bytes).unwrap();
        return;
    }
    if std::env::var("STARTUPTEST").is_ok() {
        startup_cells();
        return;
    }
    if let Ok(file) = std::env::var("TITLES_DUMP") {
        titles_dump(&file);
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
    scene_on("titles", video::TITLE, 200, false, no_keys, frames);
}
