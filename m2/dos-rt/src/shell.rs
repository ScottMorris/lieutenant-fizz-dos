// The shell around the sim: what the web build's TypeScript does with the sim's events, for the DOS build.
//
// It keeps the map prompt, the toast, the floating caption words and the cards (level cleared, a fall, game over); the sim
// waits behind a card until Enter is pressed. All text is drawn over the finished frame (see `ui`).

use core::fmt::Write;
use core::ptr::addr_of_mut;

use lf_episode_1::state::{
    AMMO, BOSS_HP, HAS_USB, KEY_BLUE, KEY_GREEN, KEY_RED, LIVES, SCORE,
};
use lf_episode_1::state::{ATTRACT_PERIOD, ATTRACT_T};
use lf_episode_1::{
    enter_map, events_clear, events_len, events_ptr, game_new, load_attract, mode, state_get, step,
};

use crate::ui::{self, Buf};
use crate::ui_data::{
    BOSS_DEFEATED, BOSS_INTRO, CAPTIONS, LEVEL_BLURBS, LEVEL_CLEARED, LEVEL_NAMES, SIGNS, TOASTS,
};
use crate::video::{H, W};

extern "C" {
    /// The sound driver (C): plays the effect for a sim caption id, or one effect by number.
    fn snd_caption(id: u32);
    fn snd_fx(effect: i32);
    fn snd_set_music(on: i32);
    fn snd_set_fx(on: i32);
    fn snd_music_enabled() -> i32;
    fn snd_fx_enabled() -> i32;
}

/// Effect numbers for the interface (the order of the sound data's effects): a click as the cursor moves, the menu sound
/// for a choice and a soft plink to go back.
const FX_MOVE: i32 = 12;
const FX_SELECT: i32 = 30;
const FX_BACK: i32 = 8;

/// Input bit for Escape, which the C side adds.
const MENU_KEY: u32 = 256;
const UP: u32 = 4;
const DOWN: u32 = 8;
const LEFT: u32 = 1;
const RIGHT: u32 = 2;
const JUMP: u32 = 16;
const FIRE: u32 = 64;

/// Input bit for Enter.
const CONFIRM: u32 = 128;
/// The saucer is the last level; it has no goal, so leaving it goes straight back to the map.
const SAUCER: f32 = 15.0;
/// Ticks to wait after a level is cleared, a fall or the end of the game before the card comes up.
const CLEARED_WAIT: u32 = 90;
const DIED_WAIT: u32 = 70;
const GAME_OVER_WAIT: u32 = 100;
/// How long a toast stays, and how long a caption word floats, in ticks.
const TOAST_TICKS: u32 = 200;
const CAPTION_TICKS: u32 = 45;
/// Ticks a card ignores Enter after it appears, so the press that finished the level does not dismiss it.
const CARD_ARM: u32 = 25;
/// The same caption is shown at most this often, as in the web build (180 ms).
const CAPTION_GAP: u32 = 11;

#[derive(Clone, Copy)]
enum Card {
    Cleared(u8),
    Died { lives: i32, level: u8 },
    GameOver(i32),
}

#[derive(Clone, Copy)]
struct Caption {
    id: u16,
    x: f32,
    y: f32,
    age: u32,
}

/// What is on the screen: the title with the attract loop behind it, the game, or a menu over the game.
#[derive(Clone, Copy, PartialEq)]
enum Screen {
    Title,
    Play,
    Pause,
    Options(Back),
}

/// Where Options goes back to.
#[derive(Clone, Copy, PartialEq)]
enum Back {
    Title,
    Pause,
}

/// What a fade to black leads to.
#[derive(Clone, Copy, PartialEq)]
enum Next {
    Title,
    NewGame,
    Map,
    Quit,
}

#[derive(Clone, Copy)]
struct Fade {
    out: bool,
    t: u32,
    then: Next,
}

/// Ticks a fade out or in takes.
const FADE_TICKS: u32 = 12;
/// The attract loop shows each of the title's three levels in turn.
const ATTRACT_LEVELS: u32 = 3;

struct Shell {
    screen: Screen,
    cursor: usize,
    fade: Option<Fade>,
    quit: bool,
    card: Option<Card>,
    card_age: u32,
    pending: Option<(Card, u32)>,
    /// The map prompt: its kind, id and extra number, as the sim sends them.
    prompt: Option<(u8, i32, i32)>,
    toast: Option<(u16, u32)>,
    captions: [Option<Caption>; 6],
    seen: [u32; CAPTIONS.len()],
    prev_held: u32,
    last_mode: u32,
    attract: u32,
    tick: u32,
}

static mut SHELL: Shell = Shell {
    screen: Screen::Play,
    cursor: 0,
    fade: None,
    quit: false,
    card: None,
    card_age: 0,
    pending: None,
    prompt: None,
    toast: None,
    captions: [None; 6],
    seen: [0; CAPTIONS.len()],
    prev_held: 0,
    last_mode: 0,
    attract: 0,
    tick: 0,
};

#[allow(static_mut_refs)]
fn sh() -> &'static mut Shell {
    // SAFETY: the shell is touched only from the game's own thread, never from an interrupt handler.
    unsafe { &mut *addr_of_mut!(SHELL) }
}

/// Forgets everything on screen: a new level, the map, a new game.
pub fn reset() {
    let s = sh();
    s.card = None;
    s.pending = None;
    s.prompt = None;
    s.toast = None;
    s.captions = [None; 6];
}

/// The game is on: a level or the map was entered by hand.
pub fn set_play() {
    reset();
    let s = sh();
    s.screen = Screen::Play;
    s.fade = None;
}

/// The title: the attract loop, with the menu over it.
pub fn start_title() {
    reset();
    let s = sh();
    s.screen = Screen::Title;
    s.cursor = 0;
    s.attract = 0;
    s.fade = Some(Fade { out: false, t: 0, then: Next::Title });
    load_attract(0, 0);
}

/// A check: puts a menu on the screen (1 pause, 2 options from the pause menu).
pub fn show(n: u32) {
    let s = sh();
    s.cursor = 0;
    s.screen = match n {
        1 => Screen::Pause,
        2 => Screen::Options(Back::Pause),
        _ => Screen::Play,
    };
}

/// Whether the player chose to leave the game.
pub fn quit_requested() -> bool {
    sh().quit
}

/// Whether a card is up, so the sim is waiting.
pub fn card_up() -> bool {
    sh().card.is_some()
}

fn goto_map() {
    crate::video::goto_map();
}

fn night_flag() -> bool {
    crate::video::night()
}

/// The effect for the menu: a click, a choice or a step back.
fn cue(effect: i32) {
    unsafe { snd_fx(effect) };
}

/// What the keys did in a menu this tick.
enum Pick {
    Nothing,
    Chosen,
    Back,
    Left,
    Right,
}

/// Moves the cursor in a menu of `len` rows from the keys that were pressed this tick.
fn menu_keys(s: &mut Shell, edges: u32, len: usize) -> Pick {
    if edges & UP != 0 {
        s.cursor = (s.cursor + len - 1) % len;
        cue(FX_MOVE);
    }
    if edges & DOWN != 0 {
        s.cursor = (s.cursor + 1) % len;
        cue(FX_MOVE);
    }
    if edges & (CONFIRM | JUMP | FIRE) != 0 {
        return Pick::Chosen;
    }
    if edges & MENU_KEY != 0 {
        return Pick::Back;
    }
    if edges & LEFT != 0 {
        return Pick::Left;
    }
    if edges & RIGHT != 0 {
        return Pick::Right;
    }
    Pick::Nothing
}

fn fade_to(s: &mut Shell, then: Next) {
    s.fade = Some(Fade { out: true, t: 0, then });
}

/// Does what a finished fade out led to.
fn perform(s: &mut Shell, then: Next) {
    match then {
        Next::Title => {
            game_new();
            start_title();
        }
        Next::NewGame => {
            game_new();
            goto_map();
        }
        Next::Map => goto_map(),
        Next::Quit => s.quit = true,
    }
}

/// One tick: the screen's own input, then (unless something waits) the sim.
pub fn advance(held: u32) {
    let s = sh();
    s.tick += 1;
    let edges = held & !s.prev_held;
    s.prev_held = held;
    let sim_held = held & 0xFF;

    // A fade out holds everything still until it is black; a fade in lets the screen run.
    if let Some(mut f) = s.fade {
        f.t += 1;
        if f.t >= FADE_TICKS {
            if f.out {
                s.fade = None;
                perform(s, f.then);
                if !s.quit {
                    s.fade = Some(Fade { out: false, t: 0, then: f.then });
                }
            } else {
                s.fade = None;
            }
            return;
        }
        s.fade = Some(f);
        if f.out {
            return;
        }
    }

    match s.screen {
        Screen::Title => title_tick(s, edges),
        Screen::Pause => pause_tick(s, edges),
        Screen::Options(back) => options_tick(s, edges, back),
        Screen::Play => play_tick(s, held, edges, sim_held),
    }
}

const TITLE_ROWS: usize = 3;
fn title_tick(s: &mut Shell, edges: u32) {
    // The attract loop: the sim steps by itself, and each level is shown for one pan across and back.
    step(0);
    let _ = events_clear();
    if state_get(ATTRACT_T) >= state_get(ATTRACT_PERIOD) {
        s.attract = (s.attract + 1) % ATTRACT_LEVELS;
        load_attract(s.attract, 0);
    }
    match menu_keys(s, edges, TITLE_ROWS) {
        Pick::Chosen => {
            cue(FX_SELECT);
            match s.cursor {
                0 => fade_to(s, Next::NewGame),
                1 => {
                    s.screen = Screen::Options(Back::Title);
                    s.cursor = 0;
                }
                _ => fade_to(s, Next::Quit),
            }
        }
        _ => {}
    }
}

fn pause_tick(s: &mut Shell, edges: u32) {
    // In a level the menu also offers the way back to the map.
    let in_level = mode() == 3;
    match menu_keys(s, edges, if in_level { 4 } else { 3 }) {
        Pick::Back => {
            cue(FX_BACK);
            s.screen = Screen::Play;
        }
        Pick::Chosen => {
            cue(FX_SELECT);
            match s.cursor {
                0 => s.screen = Screen::Play,
                1 => {
                    s.screen = Screen::Options(Back::Pause);
                    s.cursor = 0;
                }
                2 if in_level => {
                    s.screen = Screen::Play;
                    fade_to(s, Next::Map);
                }
                _ => {
                    s.screen = Screen::Play;
                    fade_to(s, Next::Title);
                }
            }
        }
        _ => {}
    }
}

const OPTION_ROWS: usize = 4;
fn options_tick(s: &mut Shell, edges: u32, back: Back) {
    let go_back = |s: &mut Shell| {
        cue(FX_BACK);
        s.screen = if back == Back::Title { Screen::Title } else { Screen::Pause };
        s.cursor = 1;
    };
    match menu_keys(s, edges, OPTION_ROWS) {
        Pick::Back => go_back(s),
        Pick::Chosen | Pick::Left | Pick::Right => {
            if s.cursor == 3 {
                go_back(s);
                return;
            }
            cue(FX_SELECT);
            match s.cursor {
                0 => unsafe { snd_set_music(i32::from(snd_music_enabled() == 0)) },
                1 => unsafe { snd_set_fx(i32::from(snd_fx_enabled() == 0)) },
                _ => crate::video::set_night(!night_flag()),
            }
        }
        Pick::Nothing => {}
    }
}

fn play_tick(s: &mut Shell, held: u32, edges: u32, sim_held: u32) {
    let confirm = edges & CONFIRM != 0;
    let _ = held;
    let mode_before = s.last_mode;

    if let Some(card) = s.card {
        s.card_age += 1;
        if confirm && s.card_age > CARD_ARM {
            if matches!(card, Card::GameOver(_)) {
                fade_to(s, Next::NewGame);
            } else {
                fade_to(s, Next::Map);
            }
            s.card = None;
        }
        return;
    }
    if edges & MENU_KEY != 0 && s.fade.is_none() {
        cue(FX_SELECT);
        s.screen = Screen::Pause;
        s.cursor = 0;
        return;
    }

    step(sim_held);
    events(s);
    // A level entered or left from inside the sim fades in from black.
    let now = mode();
    if mode_before != 0 && now != mode_before && s.fade.is_none() {
        s.fade = Some(Fade { out: false, t: 0, then: Next::Map });
    }
    s.last_mode = now;

    if let Some((card, left)) = s.pending {
        if left <= 1 {
            s.pending = None;
            s.card = Some(card);
            s.card_age = 0;
            s.prompt = None;
        } else {
            s.pending = Some((card, left - 1));
        }
    }
    if let Some((id, left)) = s.toast {
        s.toast = if left <= 1 { None } else { Some((id, left - 1)) };
    }
    for slot in s.captions.iter_mut() {
        if let Some(c) = slot {
            c.age += 1;
            if c.age > CAPTION_TICKS {
                *slot = None;
            }
        }
    }
}

fn events(s: &mut Shell) {
    const CAPTION: f32 = 1.0;
    const TOAST: f32 = 3.0;
    const LEVEL_COMPLETE: f32 = 4.0;
    const DIED: f32 = 5.0;
    const GAME_OVER: f32 = 6.0;
    const MAP_PROMPT: f32 = 10.0;
    const LEVEL_START: f32 = 11.0;
    let n = events_len() as usize;
    if n > 0 {
        let list = unsafe { core::slice::from_raw_parts(events_ptr(), n * 6) };
        for e in list.chunks_exact(6) {
            let (a, b, c) = (e[1], e[2], e[3]);
            if e[0] == CAPTION {
                unsafe { snd_caption(c as u32) };
                caption(s, a, b, c as usize);
            } else if e[0] == TOAST {
                s.toast = Some((a as u16, TOAST_TICKS));
            } else if e[0] == LEVEL_COMPLETE {
                if a >= SAUCER {
                    goto_map();
                    break;
                }
                s.pending = Some((Card::Cleared(a as u8), CLEARED_WAIT));
            } else if e[0] == DIED {
                s.pending = Some((Card::Died { lives: a as i32, level: b as u8 }, DIED_WAIT));
            } else if e[0] == GAME_OVER {
                s.pending = Some((Card::GameOver(a as i32), GAME_OVER_WAIT));
            } else if e[0] == MAP_PROMPT {
                s.prompt = if a == 0.0 { None } else { Some((a as u8, b as i32, c as i32)) };
            } else if e[0] == LEVEL_START {
                s.prompt = None;
            }
        }
    }
    events_clear();
}

/// Starts a floating word, unless the same one was shown a moment ago or it has no colour.
fn caption(s: &mut Shell, x: f32, y: f32, id: usize) {
    if id >= CAPTIONS.len() || CAPTIONS[id].1 == 255 {
        return;
    }
    if s.tick.wrapping_sub(s.seen[id]) < CAPTION_GAP && s.seen[id] != 0 {
        return;
    }
    s.seen[id] = s.tick;
    let free = s.captions.iter().position(Option::is_none).unwrap_or(0);
    s.captions[free] = Some(Caption { id: id as u16, x, y, age: 0 });
}

// ---------------------------------------------------------------------------------------------------------------------
// Drawing
// ---------------------------------------------------------------------------------------------------------------------

/// Draws the interface over the frame. `cam_px` is the snapped camera in pixels, as the sprites used.
pub fn overlay(frame: &mut [u8], cam_px: (i32, i32)) {
    let s = sh();
    let m = mode();
    match s.screen {
        Screen::Title => title_screen(frame, s),
        _ => {
            if m == 2 || m == 3 {
                captions(frame, s, cam_px);
                hud(frame);
                if let Some((id, _)) = s.toast {
                    toast(frame, id);
                }
                if let (2, Some((kind, id, extra))) = (m, s.prompt) {
                    prompt(frame, kind, id, extra);
                }
            }
            if let Some(card) = s.card {
                card_screen(frame, card);
            }
            match s.screen {
                Screen::Pause => {
                    ui::dim(frame, 0, 0, W as i32, H as i32, 7);
                    if m == 3 {
                        menu(
                            frame,
                            "Paused",
                            &[("Resume", ""), ("Options", ""), ("Back to the map", ""), ("Quit to title", "")],
                            s.cursor,
                        );
                    } else {
                        menu(frame, "Paused", &[("Resume", ""), ("Options", ""), ("Quit to title", "")], s.cursor);
                    }
                }
                Screen::Options(_) => {
                    ui::dim(frame, 0, 0, W as i32, H as i32, 7);
                    let on = |b: bool| if b { "On" } else { "Off" };
                    let (music, fx) = unsafe { (snd_music_enabled() != 0, snd_fx_enabled() != 0) };
                    menu(
                        frame,
                        "Options",
                        &[("Music", on(music)), ("Effects", on(fx)), ("Night", on(night_flag())), ("Back", "")],
                        s.cursor,
                    );
                }
                _ => {}
            }
        }
    }
    if let Some(f) = s.fade {
        let steps = (f.t * 13 / FADE_TICKS).min(13) as u8;
        ui::dim(frame, 0, 0, W as i32, H as i32, if f.out { steps } else { 13 - steps });
    }
}

/// A menu in a panel in the middle of the screen: a row each, the chosen one marked with a soda can and drawn in yellow.
fn menu(frame: &mut [u8], title: &str, rows: &[(&str, &str)], cursor: usize) {
    let h = ui::Panel::TOP_LEGEND + rows.len() as i32 * 16 + ui::Panel::BOTTOM;
    let (w, x, y) = (200, (W as i32 - 200) / 2, (H as i32 - h) / 2);
    let p = ui::Panel { x, y, w, h }.draw(frame, 5, 13);
    p.legend(frame, title, 5, ui::WHITE);
    draw_rows(frame, p, rows, cursor, ui::Panel::TOP_LEGEND);
}

fn draw_rows(frame: &mut [u8], p: ui::Panel, rows: &[(&str, &str)], cursor: usize, top: i32) {
    let soda = soda_id();
    for (i, (label, value)) in rows.iter().enumerate() {
        let y = p.y + top + i as i32 * 16;
        let on = i == cursor;
        if on {
            crate::video::draw_sprite_ui(frame, soda, p.left() - 4, y - 4);
        }
        let colour = if on { ui::YELLOW } else { ui::WHITE };
        let max = p.inner_w() - 22;
        ui::text_fit(frame, p.left() + 18, y, label, colour, max - ui::text_width(value) - 4);
        if !value.is_empty() {
            let vx = p.x + p.w - ui::Panel::SIDE - ui::text_width(value);
            ui::text(frame, vx, y, value, if on { ui::YELLOW } else { ui::CYAN });
        }
    }
}

/// The sprite the menus use for their cursor.
fn soda_id() -> usize {
    lf_episode_1::sprites::SPRITE_NAMES.iter().position(|n| *n == "soda").unwrap_or(0)
}

/// The title: the attract loop behind, dimmed, with the name and the menu.
fn title_screen(frame: &mut [u8], s: &Shell) {
    ui::dim(frame, 0, 0, W as i32, H as i32, 3);
    let head = ui::Panel { x: 24, y: 8, w: 272, h: 46 }.draw(frame, 5, 13);
    head.legend(frame, "Liminal HQ DOS port", 5, ui::CYAN);
    let w = ui::text_width("Lieutenant Fizz") * 2;
    ui::text_big(frame, (W as i32 - w) / 2, head.top(true) - 1, "Lieutenant Fizz", ui::WHITE, 2);
    ui::text_centred(frame, W as i32 / 2, head.top(true) + 21, "Episode 1 - The Cocoa Caper", ui::YELLOW);
    let rows = [("New game", ""), ("Options", ""), ("Quit", "")];
    let h = ui::Panel::TOP + rows.len() as i32 * 16 + ui::Panel::BOTTOM + 2;
    let p = ui::Panel { x: 72, y: 78, w: 176, h }.draw(frame, 3, 11);
    draw_rows(frame, p, &rows, s.cursor, ui::Panel::TOP + 4);
    ui::text_centred(frame, W as i32 / 2, 188, "Up and Down choose, Enter selects", ui::GREY);
}

fn captions(frame: &mut [u8], s: &Shell, cam_px: (i32, i32)) {
    for c in s.captions.iter().flatten() {
        let (text, colour) = CAPTIONS[usize::from(c.id)];
        let sx = W as i32 / 2 + round(c.x * 16.0) - cam_px.0;
        let sy = H as i32 / 2 - round(c.y * 16.0) + cam_px.1 - (c.age / 3) as i32;
        ui::text_centred(frame, sx, sy - 14, text, colour);
    }
}

fn round(v: f32) -> i32 {
    (v + 100_000.5) as i32 - 100_000
}

fn number(buf: &mut Buf, n: i32) -> &str {
    *buf = Buf::new();
    let _ = write!(buf, "{n}");
    buf.as_str()
}

/// The heads-up display: a small box in the corner with the score, lives, Fizz, the next life and the keys held, and
/// the boss's health in a box of its own at the other corner.
fn hud(frame: &mut [u8]) {
    let score = state_get(SCORE) as i32;
    let lives = (state_get(LIVES) as i32).max(0);
    let ammo = state_get(AMMO) as i32;
    let keys = [
        (state_get(KEY_RED) != 0.0, ui::RED),
        (state_get(KEY_BLUE) != 0.0, ui::BLUE),
        (state_get(KEY_GREEN) != 0.0, ui::GREEN),
    ];
    let usb = state_get(HAS_USB) != 0.0;
    let any_key = usb || keys.iter().any(|k| k.0);
    let rows = if any_key { 4 } else { 3 };
    let (x, y, w) = (5, 5, 124);
    let h = 6 + rows * 10 + 4;
    ui::dim(frame, x, y, w, h, 9);
    ui::outline(frame, x - 1, y - 1, w + 2, h + 2, ui::BLACK);
    ui::outline(frame, x, y, w, h, ui::GREY);
    let mut buf = Buf::new();
    let tx = x + 6;
    let mut ty = y + 5;

    let ex = ui::text(frame, tx, ty, "Score ", ui::GREY);
    *&mut buf = Buf::new();
    let _ = write!(buf, "{score:06}");
    ui::text(frame, ex, ty, buf.as_str(), ui::YELLOW);
    ty += 10;
    let ex = ui::text(frame, tx, ty, "Lives ", ui::GREY);
    let ex = ui::text(frame, ex, ty, number(&mut buf, lives), ui::GREEN);
    let ex = ui::text(frame, ex + 8, ty, "Fizz ", ui::GREY);
    ui::text(frame, ex, ty, number(&mut buf, ammo), ui::CYAN);
    ty += 10;
    let mut next = Buf::new();
    let _ = write!(next, "Next life at {}", (score / 100 + 1) * 100);
    ui::text(frame, tx, ty, next.as_str(), ui::GREY);
    ty += 10;
    if any_key {
        // The gumdrop keys held, as small coloured blocks, and the gold USB.
        let mut kx = tx;
        for (held, colour) in keys {
            if held {
                ui::fill(frame, kx, ty + 1, 7, 6, colour);
                ui::outline(frame, kx - 1, ty, 9, 8, ui::BLACK);
                kx += 12;
            }
        }
        if usb {
            ui::text(frame, kx, ty, "USB", ui::YELLOW);
        }
    }
    // The boss's health: three pips.
    let hp = state_get(BOSS_HP) as i32;
    if mode() == 3 && hp > 0 {
        let mut pips = Buf::new();
        for i in 0..3 {
            let _ = pips.write_str(if i < hp { "\u{25CF}" } else { "\u{25CB}" });
        }
        let name = "Cocoa Colossus ";
        let tw = ui::text_width(name) + ui::text_width(pips.as_str());
        let (bw, bh) = (tw + 12, 20);
        let bx = W as i32 - bw - 5;
        ui::dim(frame, bx, 5, bw, bh, 9);
        ui::outline(frame, bx - 1, 4, bw + 2, bh + 2, ui::BLACK);
        ui::outline(frame, bx, 5, bw, bh, ui::GREY);
        let end = ui::text(frame, bx + 6, 10, name, ui::RED);
        ui::text(frame, end, 10, pips.as_str(), ui::YELLOW);
    }
}

fn toast(frame: &mut [u8], id: u16) {
    let text = TOASTS[usize::from(id).min(TOASTS.len() - 1)];
    let mut lines = [(0, 0); 4];
    let n = ui::wrap(text, 290, &mut lines);
    let h = n as i32 * ui::LINE + 11;
    ui::dim(frame, 8, 24, W as i32 - 16, h, 9);
    ui::outline(frame, 8, 24, W as i32 - 16, h, ui::DARK);
    ui::paragraph(frame, 15, 29, 290, text, ui::WHITE);
}

/// The panel at the foot of the map for the marker Ben is standing at.
fn prompt(frame: &mut [u8], kind: u8, id: i32, extra: i32) {
    let level = |i: i32| LEVEL_NAMES[(i.max(0) as usize).min(LEVEL_NAMES.len() - 1)];
    let mut text = Buf::new();
    let (title, action): (&str, Option<&str>) = match kind {
        1 => {
            let i = (id.max(0) as usize).min(LEVEL_NAMES.len() - 1);
            let _ = write!(text, "{}{}", if extra == 1 { "Cleared - " } else { "" }, LEVEL_BLURBS[i]);
            (LEVEL_NAMES[i], Some("Enter"))
        }
        4 => {
            let _ = write!(text, "Locked until you clear {}.", level(id));
            (level(extra), None)
        }
        2 => {
            if extra == 1 {
                let _ = write!(text, "Humming and ready.");
                ("Teleporter", Some("Teleport"))
            } else {
                let _ = write!(text, "Quiet for now. Clear {} to power it.", level(id));
                ("Teleporter", None)
            }
        }
        5 => {
            let _ = write!(
                text,
                "Humming faintly, and going nowhere. Somewhere, something is hiding its other half."
            );
            ("A dormant teleporter", None)
        }
        3 => {
            let _ = write!(text, "Parked and steaming gently. Step inside for a look around.");
            ("Spaghetti with meatballs flying saucer", Some("Enter"))
        }
        6 => {
            let _ = write!(text, "{}", SIGNS[(id.max(0) as usize).min(SIGNS.len() - 1)]);
            ("Sign", None)
        }
        _ => return,
    };
    let mut lines = [(0, 0); 6];
    let n = ui::wrap(text.as_str(), 294, &mut lines) as i32;
    let h = 13 + ui::LINE + n * ui::LINE + if action.is_some() { ui::LINE } else { 0 };
    let y = H as i32 - 4 - h;
    ui::dim(frame, 6, y, W as i32 - 12, h, 9);
    ui::outline(frame, 6, y, W as i32 - 12, h, ui::DARK);
    ui::text(frame, 13, y + 6, title, ui::YELLOW);
    ui::paragraph(frame, 13, y + 6 + ui::LINE, 294, text.as_str(), ui::WHITE);
    if let Some(a) = action {
        let mut label = Buf::new();
        let _ = write!(label, "Press Enter, Jump or Fire to {}", a.to_ascii_lowercase());
        ui::text(frame, 13, y + 6 + ui::LINE * (n + 1), label.as_str(), ui::CYAN);
    }
}

fn card_screen(frame: &mut [u8], card: Card) {
    let mut title = Buf::new();
    let mut text = Buf::new();
    let button;
    match card {
        Card::Cleared(level) => {
            let i = usize::from(level).min(LEVEL_NAMES.len() - 1);
            let _ = write!(title, "{} cleared", LEVEL_NAMES[i]);
            let _ = write!(text, "{}", LEVEL_CLEARED[i]);
            button = "Press Enter: back to the map";
        }
        Card::Died { lives, level } => {
            let i = usize::from(level).min(LEVEL_NAMES.len() - 1);
            let _ = write!(title, "Ben took a tumble");
            let _ = write!(
                text,
                "{} sends you back to the map. {} {} left - every 100 snack points earns another.",
                LEVEL_NAMES[i],
                lives,
                if lives == 1 { "life" } else { "lives" }
            );
            button = "Press Enter: back to the map";
        }
        Card::GameOver(score) => {
            let _ = write!(title, "Out of lives");
            let _ = write!(text, "You finished with {score} snack points. Press Enter to start a new game.");
            button = "Press Enter: new game";
        }
    }
    ui::dim(frame, 0, 0, W as i32, H as i32, 6);
    let mut lines = [(0, 0); 8];
    let n = ui::wrap(text.as_str(), 232, &mut lines) as i32;
    let h = 16 + ui::LINE * 2 + n * ui::LINE + 8;
    let (w, x, y) = (252, (W as i32 - 252) / 2, (H as i32 - h) / 2);
    ui::dim(frame, x, y, w, h, 14);
    ui::fill(frame, x, y, w, 1, ui::YELLOW);
    ui::outline(frame, x, y, w, h, ui::DARK);
    ui::text_centred(frame, W as i32 / 2, y + 8, title.as_str(), ui::YELLOW);
    ui::paragraph(frame, x + 10, y + 8 + ui::LINE + 4, 232, text.as_str(), ui::WHITE);
    ui::text_centred(frame, W as i32 / 2, y + h - ui::LINE - 6, button, ui::CYAN);
}

/// Unused for now: the boss's dialogue, kept so the data stays linked until the dialogue screens are written.
#[allow(dead_code)]
pub fn dialogue_len() -> usize {
    BOSS_INTRO.len() + BOSS_DEFEATED.len()
}

// `enter_map` is called through `video::goto_map`.
#[allow(dead_code)]
fn _link() {
    enter_map();
}
