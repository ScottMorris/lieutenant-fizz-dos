// The shell around the sim: what the web build's TypeScript does with the sim's events, for the DOS build.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT
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

use crate::cine::Cine;
use crate::fx;
use crate::save::{self, SLOTS};
use crate::story::{self, Press};
use crate::titles::Titles;
use crate::ui::{self, Buf};
use crate::ui_data::{
    BOSS_DEFEATED, BOSS_INTRO, CAPTIONS, CINE, END, LIFTOFF_BEAT, LEVEL_BLURBS, LEVEL_CLEARED, LEVEL_NAMES, SIGNS, TITLE_KICKER, TITLE_LINE,
    TITLE_WORDS, TOASTS,
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
    fn snd_have_opl() -> i32;
    fn snd_fx_device() -> i32;
    fn snd_set_fx_device(device: i32);
    /// The frame rate setting (C): 0 auto, 1 60 Hz, 2 30 Hz; and the rate in use (60 or 30).
    fn rate_mode() -> i32;
    fn rate_set(mode: i32);
    fn rate_active() -> i32;
    /// The detail setting (C): 0 auto, 1 high, 2 low; and whether low is in use.
    fn detail_mode() -> i32;
    fn detail_set(mode: i32);
    fn detail_active() -> i32;
    /// Writes the options to FIZZ.CFG (C), given the night setting that the game keeps.
    fn settings_save(night: i32);
}

/// Effect numbers for the interface (the order of the sound data's effects): a click as the cursor moves, the menu sound
/// for a choice and a soft plink to go back.
const FX_MOVE: i32 = 12;
const FX_SELECT: i32 = 30;
const FX_BACK: i32 = 8;
const FX_DENY: i32 = 9;

/// Input bit for Escape, which the C side adds.
const MENU_KEY: u32 = 256;
/// Input bits for F5 (quick-save) and F9 (quick-load), which the C side adds.
const QSAVE: u32 = 512;
const QLOAD: u32 = 1024;
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
/// Ticks a second: the game's sim and the shell's timers run at 60 Hz whatever the screen's rate.
const TICKS_PER_SEC: u32 = 60;
/// Ticks Ben on the title keeps facing the menu after the cursor moves (1.6 s) and waves after a pick (0.9 s), and the length of his
/// loop (14 s).
const BEN_LOOK: u32 = TICKS_PER_SEC * 8 / 5;
const BEN_WAVE: u32 = TICKS_PER_SEC * 9 / 10;
const BEN_CYCLE: u32 = 14 * TICKS_PER_SEC;
/// The title's wordmark: its top, the size of a glyph pixel and the height of its eight rows. Ben stands on it, and `title_screen`
/// draws it from the same numbers.
const TITLE_LOGO_Y: i32 = 24;
const TITLE_LOGO_SCALE: i32 = 3;
const TITLE_LOGO_H: i32 = 8 * TITLE_LOGO_SCALE;
/// How long the quick-save message stays, in ticks.
const QUICK_TICKS: u32 = 120;
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
    /// The end of the episode, with the score and the lives left.
    End { score: i32, lives: i32 },
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
    Controls(Back),
    Slots(SlotKind, Back),
    /// The ending's panels, then the credits roll.
    Ending,
    Credits,
    /// The opening cinematic: the pictures, with the letterboxed text over them.
    Opening,
    /// The opening titles: Liminal HQ presents, the wordmark and the chase, drawn whole by `titles`.
    Titles,
}

/// Where Options and the slot lists go back to.
#[derive(Clone, Copy, PartialEq)]
enum Back {
    Title,
    Pause,
}

#[derive(Clone, Copy, PartialEq)]
enum SlotKind {
    Load,
    Save,
}

/// What a fade to black leads to.
#[derive(Clone, Copy, PartialEq)]
enum Next {
    Title,
    NewGame,
    /// A new game from the title: the opening cinematic, then the map.
    Opening,
    /// The cinematic is over or was skipped: the map, with a word about where to start.
    OpeningEnd,
    Map,
    Quit,
    Load(u8),
}

#[derive(Clone, Copy)]
struct Fade {
    ramp: fx::Ramp,
    then: Next,
}

/// A line of the boss's dialogue being typed out.
#[derive(Clone, Copy)]
struct Dialogue {
    id: u8,
    line: u8,
    typed: u16,
}

#[derive(Clone, Copy, PartialEq)]
enum TitleItem {
    Continue,
    New,
    Load,
    Options,
    Controls,
    Quit,
}

#[derive(Clone, Copy, PartialEq)]
enum PauseItem {
    Resume,
    Save,
    Options,
    Controls,
    Map,
    Title,
}

#[derive(Clone, Copy, PartialEq)]
enum OptionItem {
    Music,
    Effects,
    Device,
    Rate,
    Detail,
    Night,
    Back,
}

/// Ticks a fade out or in takes.
const FADE_TICKS: u32 = 12;
/// The attract loop shows each of the title's three levels in turn.
const ATTRACT_LEVELS: u32 = 3;
/// Characters of dialogue typed a tick.
const TYPE_RATE: u16 = 2;

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
    dialogue: Option<Dialogue>,
    saves: [Option<save::Info>; SLOTS],
    /// A short message under the menus: 1 saved (the slot in `note_arg`), 2 could not save.
    note: Option<(u8, u8, u32)>,
    /// A short message over the game after F5 or F9: 1 quick-saved, 2 could not save, 3 nothing to load.
    quick: Option<(u8, u32)>,
    played: u32,
    prev_held: u32,
    /// Keys that a menu or a line of dialogue took as its own press and that are still down: the sim does not see them until they are let go.
    swallow: u32,
    last_mode: u32,
    attract: u32,
    /// The tick up to which Ben on the title faces the menu (after the cursor moves) and waves (after a pick).
    ben_look: u32,
    ben_wave: u32,
    /// The sheet ids of Ben's title poses (stand, jump, shoot, pogo, pogo2); `usize::MAX` for one the sheet lacks, which draws nothing.
    ben_ids: [usize; 5],
    reel: story::Reel,
    roll: story::Roll,
    cine: Option<Cine>,
    titles: Option<Titles>,
    /// A message over the game with its ticks left (the first words of a new game).
    hint: Option<(&'static str, u32)>,
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
    dialogue: None,
    saves: [None; SLOTS],
    note: None,
    quick: None,
    played: 0,
    prev_held: 0,
    swallow: 0,
    last_mode: 0,
    attract: 0,
    ben_look: 0,
    ben_wave: 0,
    ben_ids: [usize::MAX; 5],
    reel: story::Reel::empty(),
    roll: story::Roll::new(),
    cine: None,
    titles: None,
    hint: None,
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
    s.dialogue = None;
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
    refresh_saves(s);
    s.screen = Screen::Title;
    s.cursor = 0;
    s.attract = 0;
    s.ben_ids = ["ben_stand", "ben_jump", "ben_shoot", "ben_pogo", "ben_pogo2"].map(|n| crate::video::sprite_by_name(n).unwrap_or(usize::MAX));
    s.fade = Some(Fade { ramp: fx::Ramp::new(FADE_TICKS, false), then: Next::Title });
    load_attract(0, 0);
}

/// Whether the player chose to leave the game.
pub fn quit_requested() -> bool {
    sh().quit
}

/// A check: puts a screen up (1 pause, 2 options, 3 load list, 4 save list, 6 controls, 7 the ending, 8 the credits, 9 the end card, 100 to 107 an opening scene,
/// 200 the opening titles from their first tick).
pub fn show(n: u32) {
    let s = sh();
    s.cursor = 0;
    refresh_saves(s);
    s.screen = match n {
        1 => Screen::Pause,
        2 => Screen::Options(Back::Pause),
        3 => Screen::Slots(SlotKind::Load, Back::Pause),
        4 => Screen::Slots(SlotKind::Save, Back::Pause),
        6 => Screen::Controls(Back::Pause),
        7 => {
            start_ending(s);
            return;
        }
        8 => {
            s.roll.start();
            s.screen = Screen::Credits;
            return;
        }
        9 => {
            s.screen = Screen::Play;
            s.card = Some(Card::End { score: 1234, lives: 1 });
            return;
        }
        n if (100..108).contains(&n) => {
            // A check: the opening cinematic at a scene, `k` seconds into it (the scene's own clock runs on from there).
            start_opening(s);
            let scene = (n - 100) as usize;
            while s.reel.scene() < scene {
                let _ = s.reel.press();
            }
            if let Some(cine) = s.cine.as_mut() {
                cine.start(scene as u8);
                if scene == 2 {
                    cine.launch();
                }
            }
            return;
        }
        200 => {
            // A new game first, so that nothing of the title's attract loop is left under the titles.
            game_new();
            start_titles(s, 1);
            return;
        }
        n if (10..100).contains(&n) => {
            // A check: load a slot straight away.
            perform(s, Next::Load((n - 10) as u8));
            return;
        }
        5 => {
            s.screen = Screen::Play;
            s.dialogue = Some(Dialogue { id: 0, line: 1, typed: 80 });
            return;
        }
        _ => Screen::Play,
    };
}

/// Whether the screen draws every pixel of the frame itself (the opening titles), so the sim's scene need not be built or painted.
pub fn draws_whole_frame() -> bool {
    sh().screen == Screen::Titles
}

/// Whether a card is up, so the sim is waiting.
pub fn card_up() -> bool {
    sh().card.is_some()
}

fn refresh_saves(s: &mut Shell) {
    for i in 0..SLOTS {
        s.saves[i] = save::info(i as i32);
    }
}

fn seconds(s: &Shell) -> u32 {
    s.played / 60
}

/// Writes the autosave, as the web build does on every visit to the map.
fn autosave(s: &mut Shell) {
    save::write(0, seconds(s));
    refresh_saves(s);
}

fn to_map(s: &mut Shell) {
    crate::video::goto_map();
    autosave(s);
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
    if s.cursor >= len {
        s.cursor = len - 1;
    }
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
    s.fade = Some(Fade { ramp: fx::Ramp::new(FADE_TICKS, true), then });
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
            s.played = 0;
            to_map(s);
        }
        Next::Opening => {
            game_new();
            s.played = 0;
            start_opening(s);
        }
        Next::OpeningEnd => leave_opening(s),
        Next::Map => to_map(s),
        Next::Quit => s.quit = true,
        Next::Load(slot) => {
            game_new();
            s.played = save::load(i32::from(slot)).unwrap_or(0) * 60;
            to_map(s);
        }
    }
}

/// One tick: the screen's own input, then (unless something waits) the sim.
pub fn advance(held: u32) {
    let s = sh();
    s.tick += 1;
    let edges = held & !s.prev_held;
    s.prev_held = held;
    s.swallow &= held;
    let sim_held = held & 0xFF & !s.swallow;
    let was_play = s.screen == Screen::Play;
    if let Some((k, a, left)) = s.note {
        s.note = if left <= 1 { None } else { Some((k, a, left - 1)) };
    }
    if let Some((text, left)) = s.hint {
        s.hint = if left <= 1 { None } else { Some((text, left - 1)) };
    }
    if let Some((k, left)) = s.quick {
        s.quick = if left <= 1 { None } else { Some((k, left - 1)) };
    }
    // Not over a card, a line of dialogue or a result on its way: a save then would hold the state of a lost or finished run.
    if matches!(s.screen, Screen::Play | Screen::Pause) && s.fade.is_none() && s.card.is_none() && s.pending.is_none() && s.dialogue.is_none() {
        quick_keys(s, edges);
    }

    // A fade out holds everything still until it is black; a fade in lets the screen run.
    if let Some(mut f) = s.fade {
        f.ramp.t += 1;
        if f.ramp.done() {
            if f.ramp.out {
                s.fade = None;
                perform(s, f.then);
                if !s.quit {
                    s.fade = Some(Fade { ramp: fx::Ramp::new(FADE_TICKS, false), then: f.then });
                }
            } else {
                s.fade = None;
            }
            return;
        }
        s.fade = Some(f);
        if f.ramp.out {
            return;
        }
    }

    match s.screen {
        Screen::Title => title_tick(s, edges),
        Screen::Pause => pause_tick(s, edges),
        Screen::Options(back) => options_tick(s, edges, back),
        Screen::Controls(back) => controls_tick(s, edges, back),
        Screen::Slots(kind, back) => slots_tick(s, edges, kind, back),
        Screen::Play => play_tick(s, edges, sim_held),
        Screen::Ending => ending_tick(s, edges),
        Screen::Credits => credits_tick(s, edges),
        Screen::Opening => opening_tick(s, edges),
        Screen::Titles => titles_tick(s),
    }
    // Going back into the game from a menu: the key that chose it is still down, and must not be read as a press in the game.
    if !was_play && s.screen == Screen::Play {
        s.swallow = held & (CONFIRM | JUMP | FIRE);
    }
}

/// F5 writes the quick-save slot and F9 loads it, in a level, on the map or in the pause menu. The slot is its own: the
/// four the player picks in the pause menu are never touched.
fn quick_keys(s: &mut Shell, edges: u32) {
    if edges & QSAVE != 0 {
        let ok = save::write(save::QUICK as i32, seconds(s));
        refresh_saves(s);
        cue(if ok { FX_SELECT } else { FX_DENY });
        s.quick = Some((if ok { 1 } else { 2 }, QUICK_TICKS));
    } else if edges & QLOAD != 0 {
        if s.saves[save::QUICK].is_some() {
            cue(FX_SELECT);
            s.screen = Screen::Play;
            fade_to(s, Next::Load(save::QUICK as u8));
        } else {
            cue(FX_DENY);
            s.quick = Some((3, QUICK_TICKS));
        }
    }
}

fn title_items(s: &Shell) -> ([TitleItem; 6], usize) {
    let any = s.saves.iter().any(Option::is_some);
    let mut v = [TitleItem::New; 6];
    let mut n = 0;
    let mut push = |item| {
        v[n] = item;
        n += 1;
    };
    if any {
        push(TitleItem::Continue);
    }
    push(TitleItem::New);
    if any {
        push(TitleItem::Load);
    }
    push(TitleItem::Options);
    push(TitleItem::Controls);
    push(TitleItem::Quit);
    (v, n)
}

fn title_tick(s: &mut Shell, edges: u32) {
    // The attract loop: the sim steps by itself, and each level is shown for one pan across and back.
    step(0);
    crate::video::stepped();
    events_clear();
    if state_get(ATTRACT_T) >= state_get(ATTRACT_PERIOD) {
        s.attract = (s.attract + 1) % ATTRACT_LEVELS;
        load_attract(s.attract, 0);
    }
    let (items, n) = title_items(s);
    if edges & (UP | DOWN) != 0 {
        s.ben_look = s.tick + BEN_LOOK;
    }
    if let Pick::Chosen = menu_keys(s, edges, n) {
        cue(FX_SELECT);
        s.ben_wave = s.tick + BEN_WAVE;
        match items[s.cursor] {
            TitleItem::Continue => {
                if let Some(slot) = save::newest(&s.saves) {
                    fade_to(s, Next::Load(slot as u8));
                }
            }
            TitleItem::New => fade_to(s, Next::Opening),
            TitleItem::Load => {
                s.screen = Screen::Slots(SlotKind::Load, Back::Title);
                s.cursor = 0;
            }
            TitleItem::Options => {
                s.screen = Screen::Options(Back::Title);
                s.cursor = 0;
            }
            TitleItem::Controls => {
                s.screen = Screen::Controls(Back::Title);
                s.cursor = 0;
            }
            TitleItem::Quit => fade_to(s, Next::Quit),
        }
    }
}

fn pause_items() -> ([PauseItem; 6], usize) {
    let mut v = [PauseItem::Resume, PauseItem::Save, PauseItem::Options, PauseItem::Controls, PauseItem::Map, PauseItem::Title];
    if mode() == 3 {
        (v, 6)
    } else {
        v[4] = PauseItem::Title;
        (v, 5)
    }
}

fn pause_tick(s: &mut Shell, edges: u32) {
    let (items, n) = pause_items();
    match menu_keys(s, edges, n) {
        Pick::Back => {
            cue(FX_BACK);
            s.screen = Screen::Play;
        }
        Pick::Chosen => {
            cue(FX_SELECT);
            match items[s.cursor] {
                PauseItem::Resume => s.screen = Screen::Play,
                PauseItem::Save => {
                    refresh_saves(s);
                    s.screen = Screen::Slots(SlotKind::Save, Back::Pause);
                    s.cursor = 0;
                }
                PauseItem::Options => {
                    s.screen = Screen::Options(Back::Pause);
                    s.cursor = 0;
                }
                PauseItem::Controls => {
                    s.screen = Screen::Controls(Back::Pause);
                    s.cursor = 0;
                }
                PauseItem::Map => {
                    s.screen = Screen::Play;
                    fade_to(s, Next::Map);
                }
                PauseItem::Title => {
                    s.screen = Screen::Play;
                    fade_to(s, Next::Title);
                }
            }
        }
        _ => {}
    }
}

/// Goes back from a sub-screen to the title or the pause menu, with the cursor on the row that opened it.
fn return_to(s: &mut Shell, back: Back, title: TitleItem, pause: PauseItem) {
    cue(FX_BACK);
    if back == Back::Title {
        s.screen = Screen::Title;
        s.cursor = title_items(s).0.iter().position(|i| *i == title).unwrap_or(0);
    } else {
        s.screen = Screen::Pause;
        s.cursor = pause_items().0.iter().position(|i| *i == pause).unwrap_or(0);
    }
}

fn controls_tick(s: &mut Shell, edges: u32, back: Back) {
    if edges & (MENU_KEY | CONFIRM) != 0 {
        return_to(s, back, TitleItem::Controls, PauseItem::Controls);
    }
}

fn option_items() -> ([OptionItem; 7], usize) {
    // The device row only makes sense with sound running and an AdLib to choose against the speaker.
    let both = unsafe { snd_have_opl() != 0 && snd_fx_device() != 0 };
    use OptionItem::*;
    if both {
        ([Music, Effects, Device, Rate, Detail, Night, Back], 7)
    } else {
        ([Music, Effects, Rate, Detail, Night, Back, Back], 6)
    }
}

fn options_tick(s: &mut Shell, edges: u32, back: Back) {
    let (items, n) = option_items();
    let leave = |s: &mut Shell| return_to(s, back, TitleItem::Options, PauseItem::Options);
    match menu_keys(s, edges, n) {
        Pick::Back => leave(s),
        Pick::Chosen | Pick::Left | Pick::Right => match items[s.cursor] {
            OptionItem::Back => leave(s),
            item => {
                change_option(item);
                // Every change is kept, so the next run starts the way this one was left.
                unsafe { settings_save(i32::from(night_flag())) }
            }
        },
        Pick::Nothing => {}
    }
}

/// Applies one press on an option row.
fn change_option(item: OptionItem) {
    match item {
        OptionItem::Back => {}
            OptionItem::Music => {
                cue(FX_SELECT);
                unsafe { snd_set_music(i32::from(snd_music_enabled() == 0)) }
            }
            OptionItem::Effects => {
                cue(FX_SELECT);
                unsafe { snd_set_fx(i32::from(snd_fx_enabled() == 0)) }
            }
            OptionItem::Device => {
                unsafe {
                    snd_set_fx_device(if snd_fx_device() == 1 { 2 } else { 1 });
                }
                cue(FX_SELECT);
            }
            OptionItem::Rate => {
                cue(FX_SELECT);
                unsafe { rate_set((rate_mode() + 1) % 3) }
            }
            OptionItem::Detail => {
                cue(FX_SELECT);
                unsafe { detail_set((detail_mode() + 1) % 3) }
            }
            OptionItem::Night => {
                cue(FX_SELECT);
                crate::video::set_night(!night_flag());
            }
    }
}

/// How many rows a slot list has, and which slot a row is.
fn slot_rows(kind: SlotKind) -> (usize, usize) {
    // (first slot, number of slots): a load list starts at the autosave; a save list cannot write to it.
    if kind == SlotKind::Load {
        (0, SLOTS)
    } else {
        (1, save::QUICK - 1)
    }
}

fn slots_tick(s: &mut Shell, edges: u32, kind: SlotKind, back: Back) {
    let (first, count) = slot_rows(kind);
    let leave = |s: &mut Shell| return_to(s, back, TitleItem::Load, PauseItem::Save);
    match menu_keys(s, edges, count + 1) {
        Pick::Back => leave(s),
        Pick::Chosen => {
            if s.cursor == count {
                leave(s);
                return;
            }
            let slot = first + s.cursor;
            if kind == SlotKind::Load {
                if s.saves[slot].is_some() {
                    cue(FX_SELECT);
                    fade_to(s, Next::Load(slot as u8));
                } else {
                    cue(FX_DENY);
                }
            } else {
                let ok = save::write(slot as i32, seconds(s));
                refresh_saves(s);
                cue(if ok { FX_SELECT } else { FX_DENY });
                s.note = Some((if ok { 1 } else { 2 }, slot as u8, 120));
            }
        }
        _ => {}
    }
}

fn play_tick(s: &mut Shell, edges: u32, sim_held: u32) {
    let confirm = edges & (CONFIRM | JUMP | FIRE) != 0;
    let mode_before = s.last_mode;
    s.played += 1;

    if let Some(card) = s.card {
        s.card_age += 1;
        if edges & CONFIRM != 0 && s.card_age > CARD_ARM && s.fade.is_none() {
            if matches!(card, Card::GameOver(_)) {
                fade_to(s, Next::NewGame);
                s.card = None;
            } else if matches!(card, Card::End { .. }) {
                // The card stays up through the fade, so the ending's song plays until the title takes over.
                fade_to(s, Next::Title);
            } else {
                fade_to(s, Next::Map);
                s.card = None;
            }
        }
        return;
    }
    // Dialogue: the sim waits while the words are typed, and a press finishes a line, then moves to the next.
    if let Some(mut d) = s.dialogue {
        let lines = if d.id == 0 { BOSS_INTRO } else { BOSS_DEFEATED };
        let len = lines.get(usize::from(d.line)).map_or(0, |l| l.1.chars().count()) as u16;
        d.typed = (d.typed + TYPE_RATE).min(len);
        if confirm {
            if d.typed < len {
                d.typed = len;
            } else if usize::from(d.line) + 1 < lines.len() {
                d.line += 1;
                d.typed = 0;
                cue(FX_MOVE);
            } else {
                s.dialogue = None;
                s.swallow = s.prev_held & (CONFIRM | JUMP | FIRE);
                cue(FX_SELECT);
                return;
            }
        }
        s.dialogue = Some(d);
        return;
    }
    if edges & MENU_KEY != 0 && s.fade.is_none() {
        cue(FX_SELECT);
        s.screen = Screen::Pause;
        s.cursor = 0;
        return;
    }

    step(sim_held);
    crate::video::stepped();
    crate::video::mark(8);
    events(s);
    crate::video::mark(9);
    // A level entered or left from inside the sim fades in from black.
    let now = mode();
    if mode_before != 0 && now != mode_before && s.fade.is_none() {
        s.fade = Some(Fade { ramp: fx::Ramp::new(FADE_TICKS, false), then: Next::Map });
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
    const DIALOGUE: f32 = 7.0;
    const ENDING: f32 = 9.0;
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
                    to_map(s);
                    break;
                }
                s.pending = Some((Card::Cleared(a as u8), CLEARED_WAIT));
            } else if e[0] == DIED {
                s.pending = Some((Card::Died { lives: a as i32, level: b as u8 }, DIED_WAIT));
            } else if e[0] == GAME_OVER {
                s.pending = Some((Card::GameOver(a as i32), GAME_OVER_WAIT));
            } else if e[0] == DIALOGUE {
                s.dialogue = Some(Dialogue { id: a as u8, line: 0, typed: 0 });
            } else if e[0] == ENDING {
                start_ending(s);
                break;
            } else if e[0] == MAP_PROMPT {
                s.prompt = if a == 0.0 { None } else { Some((a as u8, b as i32, c as i32)) };
            } else if e[0] == LEVEL_START {
                s.prompt = None;
            }
        }
    }
    events_clear();
}

/// The sim has finished the episode: the closing panels, the credits, then the end card.
fn start_ending(s: &mut Shell) {
    reset();
    s.fade = None;
    s.reel.start(END);
    s.screen = Screen::Ending;
    s.card_age = 0;
}

fn ending_tick(s: &mut Shell, edges: u32) {
    s.reel.tick();
    // The presses that were still going when the sim ended the game must not turn the first page.
    s.card_age += 1;
    if edges & (CONFIRM | JUMP | FIRE) != 0 && s.card_age > CARD_ARM {
        match s.reel.press() {
            Press::End => {
                cue(FX_SELECT);
                s.roll.start();
                s.screen = Screen::Credits;
            }
            Press::Complete => {}
            Press::Page | Press::Scene => cue(FX_MOVE),
        }
    }
}

fn credits_tick(s: &mut Shell, edges: u32) {
    s.roll.tick();
    let leave = if edges & MENU_KEY != 0 {
        true
    } else if edges & (CONFIRM | JUMP | FIRE) != 0 {
        cue(FX_MOVE);
        s.roll.press()
    } else {
        false
    };
    if leave {
        cue(FX_SELECT);
        // The end card sits over the last frame of the game, and a press of Enter takes it back to the title.
        s.screen = Screen::Play;
        s.card = Some(Card::End { score: state_get(SCORE) as i32, lives: state_get(LIVES) as i32 });
        s.card_age = 0;
    }
}

/// A new game from the title: the cinematic, which ends on the map.
fn start_opening(s: &mut Shell) {
    reset();
    s.reel.start(CINE);
    let mut cine = Cine::new();
    cine.start(0);
    s.cine = Some(cine);
    s.screen = Screen::Opening;
}

/// The opening titles, `loops` times over.
fn start_titles(s: &mut Shell, loops: u8) {
    reset();
    s.fade = None;
    s.titles = Some(Titles::new(loops));
    s.screen = Screen::Titles;
}

/// The titles run on their own clock and keys do nothing; they fade themselves out, so the title menu follows straight on.
fn titles_tick(s: &mut Shell) {
    let more = s.titles.as_mut().is_some_and(|t| t.tick(0));
    if !more {
        s.titles = None;
        perform(s, Next::Title);
    }
}

/// The cinematic is over, or was skipped: the map, with a word about where to go.
fn leave_opening(s: &mut Shell) {
    s.cine = None;
    s.screen = Screen::Play;
    to_map(s);
    // The main game's words for the same moment (`skipCine` in game.ts), which it does not keep as data.
    s.hint = Some(("Find Billy. Start at the Crater Fields, along the path to the east.", 360));
}

fn opening_tick(s: &mut Shell, edges: u32) {
    s.reel.tick();
    if let Some(cine) = s.cine.as_mut() {
        cine.tick();
    }
    if edges & MENU_KEY != 0 {
        cue(FX_BACK);
        fade_to(s, Next::OpeningEnd);
        return;
    }
    if edges & (CONFIRM | JUMP | FIRE) != 0 {
        let scene = s.reel.scene();
        match s.reel.press() {
            Press::End => {
                cue(FX_SELECT);
                fade_to(s, Next::OpeningEnd);
                return;
            }
            Press::Complete => {}
            Press::Page | Press::Scene => cue(FX_MOVE),
        }
        if let Some(cine) = s.cine.as_mut() {
            if s.reel.scene() != scene {
                cine.start(s.reel.scene() as u8);
            }
            // The launch waits for the page that opens the hatch.
            if s.reel.scene() == 2 && s.reel.first_beat() >= LIFTOFF_BEAT {
                cine.launch();
            }
        }
    }
}

/// The song a story screen wants, so the C side can play it: 1 is the ending's, 2 to 5 the opening's, 6 the titles', 0 leaves the choice to
/// the game.
pub fn music_key() -> u32 {
    let s = sh();
    match (s.screen, s.card) {
        (Screen::Ending | Screen::Credits, _) | (_, Some(Card::End { .. })) => 1,
        // The yard, the lab, the liftoff and the flight: the main game's `CINE_TRACK`, as positions in the extra songs that
        // `m4/export-sound.ts` writes after the ending (1: ending, 2 yard, 3 lab, 4 launch, 5 cine).
        (Screen::Opening, _) => [2, 3, 4, 4, 5, 5, 5, 5][s.reel.scene().min(7)],
        (Screen::Titles, _) => crate::sound_data::EXTRA_TITLES as u32 + 1,
        _ => 0,
    }
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
        Screen::Ending => s.reel.draw(frame, s.tick, false),
        Screen::Credits => s.roll.draw(frame),
        Screen::Opening => {
            if let Some(cine) = s.cine.as_ref() {
                cine.draw(frame);
            }
            s.reel.draw(frame, s.tick, true);
        }
        Screen::Titles => {
            if let Some(t) = s.titles.as_ref() {
                t.draw(frame);
            }
        }
        _ => {
            if m == 2 || m == 3 {
                captions(frame, s, cam_px);
                if s.screen == Screen::Play {
                    hud(frame);
                }
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
            if let Some((text, _)) = s.hint {
                toast_text(frame, text);
            }
            if let Some(d) = s.dialogue {
                dialogue_box(frame, d, s.tick);
            }
            match s.screen {
                Screen::Pause => pause_screen(frame, s),
                Screen::Options(_) => options_screen(frame, s),
                Screen::Controls(_) => controls_screen(frame),
                Screen::Slots(kind, _) => slots_screen(frame, s, kind),
                _ => {}
            }
            // After the menus, so the pause screen's dim does not reach it, and above its hint line.
            if let (Screen::Play | Screen::Pause, Some((kind, _))) = (s.screen, s.quick) {
                let (text, ink) = match kind {
                    1 => ("Quick-saved", ui::GREEN),
                    2 => ("Could not write QUICK.SAV", ui::RED),
                    _ => ("No quick-save yet: F5 makes one", ui::RED),
                };
                let w = ui::text_width(text) + 14;
                ui::dim(frame, (W as i32 - w) / 2, 164, w, 14, 11);
                ui::text_centred(frame, W as i32 / 2, 168, text, ink);
            }
        }
    }
    if let Some(f) = s.fade {
        fx::fade(frame, f.ramp.steps());
    }
}

/// The game's menu rows: a bright plate behind the chosen one in dark ink, the soda can bobbing in the gutter, and white text with a
/// shadow for the rest. `x` is the left of the gutter, `w` the whole width.
fn plate_rows(frame: &mut [u8], x: i32, y: i32, w: i32, rows: &[(&str, &str)], cursor: usize, tick: u32) {
    let soda = soda_id();
    let (px, pw) = (x + 18, w - 18);
    for (i, (label, value)) in rows.iter().enumerate() {
        let ry = y + i as i32 * 14;
        let on = i == cursor;
        let vw = ui::text_width(value);
        if on {
            // The plate, with its corners clipped.
            ui::fill(frame, px + 1, ry, pw - 2, 12, ui::YELLOW);
            ui::fill(frame, px, ry + 1, pw, 10, ui::YELLOW);
            crate::video::draw_sprite_ui(frame, soda, x, ry - 2 - ((tick / 20) & 1) as i32);
            let end = ui::text_ink(frame, px + 6, ry + 2, label, ui::BLACK);
            let _ = end;
            if vw > 0 {
                ui::text_ink(frame, px + pw - 6 - vw, ry + 2, value, ui::BLACK);
            }
        } else {
            ui::text_fit(frame, px + 6, ry + 2, label, ui::WHITE, pw - 14 - vw);
            if vw > 0 {
                ui::text(frame, px + pw - 6 - vw, ry + 2, value, ui::CYAN);
            }
        }
    }
}

/// The sprite the menus use for their cursor.
fn soda_id() -> usize {
    crate::video::sprite_by_name("soda").unwrap_or(0)
}

fn heading(frame: &mut [u8], s: &str, y: i32) {
    let w = ui::text_width(s) * 2;
    ui::text_big(frame, (W as i32 - w) / 2, y, s, ui::WHITE, 2);
}

/// Ben on the wordmark, as the main game's `benFrame` has him on a 14 second loop: he stands at the end of the logo with a bob, leans
/// aside now and then, hops up and pogoes across the top and back, turns to face the menu when the cursor moves and waves when an
/// item is picked. The lean turns the sprite about its feet, and the sizes are those of a 16-pixel sprite at 1×. `left` and
/// `logo_w` place the wordmark.
fn title_ben(frame: &mut [u8], s: &Shell, left: i32, logo_w: i32) {
    // The curve sin(k·π) over 0..1, as a parabola.
    fn arc(k: f32) -> f32 {
        4.0 * k * (1.0 - k)
    }
    let bw = 16.0f32;
    let home = (logo_w + 6) as f32;
    let top = TITLE_LOGO_H as f32 * 0.5;
    let p = (s.tick % BEN_CYCLE) as f32 / TICKS_PER_SEC as f32;
    let looking = s.ben_look > s.tick;
    // A wave is for when Ben is at home; while he is across the wordmark it waits, so he does not jump back and forth.
    let waving = s.ben_wave > s.tick && !(9.0..13.0).contains(&p);
    // (pose index into `ben_ids`, x from the wordmark's left, feet above its bottom, faces left, lean in radians)
    let (pose, x, y, flip, lean) = if waving {
        let k = 1.0 - (s.ben_wave - s.tick) as f32 / BEN_WAVE as f32;
        (2, home, arc(k) * bw * 0.4, true, 0.0)
    } else if (4.5..6.5).contains(&p) {
        (0, home - bw / 6.0, 0.0, looking, -12.0 * 0.017_453_3)
    } else if (9.0..13.0).contains(&p) {
        let start = home - bw;
        if p < 9.4 {
            let k = (p - 9.0) / 0.4;
            (1, home + (start - home) * k, top * k + arc(k) * bw * 0.5, true, 0.0)
        } else if p < 12.6 {
            let q = if p < 11.0 { (p - 9.4) / 1.6 } else { 1.0 - (p - 11.0) / 1.6 };
            let u = (p - 9.4) * 1.25;
            let hop = arc(u - (u as i32) as f32);
            (if hop < 0.25 { 4 } else { 3 }, start * (1.0 - q), top + hop * bw * 0.3, p < 11.0, 0.0)
        } else {
            let k = (p - 12.6) / 0.4;
            (1, start + (home - start) * k, top * (1.0 - k) + arc(k) * bw * 0.4, false, 0.0)
        }
    } else {
        (0, home, if ((p * 1.5) as i32) % 2 == 1 { 1.0 } else { 0.0 }, looking, 0.0)
    };
    let id = s.ben_ids[pose];
    let feet = TITLE_LOGO_Y + TITLE_LOGO_H - y.max(0.0) as i32;
    // Kept on the screen if a longer title pushes the wordmark's end out.
    let at = (left + x as i32).min(W as i32 - bw as i32);
    crate::video::draw_sprite_ui_lean(frame, id, at, feet - crate::video::sprite_height(id), flip, lean);
}

/// The title: the attract loop behind, dimmed, with the name and the menu.
fn title_screen(frame: &mut [u8], s: &Shell) {
    ui::dim(frame, 0, 0, W as i32, H as i32, 5);
    let w = ui::text_width(TITLE_WORDS) * TITLE_LOGO_SCALE;
    ui::text_logo(frame, (W as i32 - w) / 2, TITLE_LOGO_Y, TITLE_WORDS, ui::WHITE, ui::RED, TITLE_LOGO_SCALE);
    ui::text_centred(frame, W as i32 / 2, 12, TITLE_KICKER, ui::YELLOW);
    ui::text_centred(frame, W as i32 / 2, 56, TITLE_LINE, ui::CYAN);
    // Ben is drawn over the words, so his pogo stick crosses in front of them.
    title_ben(frame, s, (W as i32 - w) / 2, w);
    let (items, n) = title_items(s);
    let mut rows: [(&str, &str); 6] = [("", ""); 6];
    for i in 0..n {
        rows[i] = (
            match items[i] {
                TitleItem::Continue => "Continue",
                TitleItem::New => "New game",
                TitleItem::Load => "Load game",
                TitleItem::Options => "Options",
                TitleItem::Controls => "Controls",
                TitleItem::Quit => "Quit",
            },
            "",
        );
    }
    plate_rows(frame, 85, 82, 150, &rows[..n], s.cursor, s.tick);
    ui::text_centred(frame, W as i32 / 2, 188, "Up and Down choose, Enter selects", ui::GREY);
}

fn pause_screen(frame: &mut [u8], s: &Shell) {
    ui::dim(frame, 0, 0, W as i32, H as i32, 7);
    heading(frame, "Paused", 30);
    let (items, n) = pause_items();
    let mut rows: [(&str, &str); 6] = [("", ""); 6];
    for i in 0..n {
        rows[i] = (
            match items[i] {
                PauseItem::Resume => "Resume",
                PauseItem::Save => "Save game",
                PauseItem::Options => "Options",
                PauseItem::Controls => "Controls",
                PauseItem::Map => "Back to the map",
                PauseItem::Title => "Quit to title",
            },
            "",
        );
    }
    plate_rows(frame, 70, 66, 180, &rows[..n], s.cursor, s.tick);
    ui::text_centred(frame, W as i32 / 2, 188, "Up and Down choose, Enter selects, Esc resumes", ui::GREY);
}

/// The keys, as the C side's key map reads them: both schemes work at once.
const CONTROLS: [(&str, &str); 8] = [
    ("Move", "Arrows or A and D"),
    ("Aim Fizz", "Up and Down, or W and S"),
    ("Jump", "Ctrl or Z"),
    ("Pogo (toggle)", "Alt or X"),
    ("Fizz", "Space or C"),
    ("Menu, and back", "Esc"),
    ("Choose in a menu", "Enter"),
    ("Quick-save, load (in play)", "F5, F9"),
];

fn controls_screen(frame: &mut [u8]) {
    ui::dim(frame, 0, 0, W as i32, H as i32, 11);
    heading(frame, "Controls", 14);
    let (x, w) = (20, W as i32 - 40);
    for (i, (action, keys)) in CONTROLS.iter().enumerate() {
        let y = 44 + i as i32 * 13;
        ui::text(frame, x, y, action, ui::WHITE);
        ui::text(frame, x + w - ui::text_width(keys), y, keys, ui::CYAN);
    }
    ui::paragraph(frame, x, 44 + CONTROLS.len() as i32 * 13 + 6, w, "Hold jump while pogoing for a high bounce. Aim Fizz up with Up, or down with Down in the air.", ui::GREY);
    ui::text_centred(frame, W as i32 / 2, 188, "Enter or Esc goes back", ui::GREY);
}

fn options_screen(frame: &mut [u8], s: &Shell) {
    ui::dim(frame, 0, 0, W as i32, H as i32, 7);
    heading(frame, "Options", 30);
    let on = |b: bool| if b { "On" } else { "Off" };
    let (music, fx) = unsafe { (snd_music_enabled() != 0, snd_fx_enabled() != 0) };
    let device = if unsafe { snd_fx_device() } == 1 { "AdLib" } else { "PC speaker" };
    let (items, n) = option_items();
    let mut rows: [(&str, &str); 7] = [("", ""); 7];
    let rate = match unsafe { (rate_mode(), rate_active()) } {
        (0, 30) => "Auto (30 Hz)",
        (0, _) => "Auto (60 Hz)",
        (1, _) => "60 Hz",
        _ => "30 Hz",
    };
    let detail = match unsafe { (detail_mode(), detail_active()) } {
        (0, 1) => "Auto (Low)",
        (0, _) => "Auto (High)",
        (1, _) => "High",
        _ => "Low",
    };
    for i in 0..n {
        rows[i] = match items[i] {
            OptionItem::Rate => ("Frame rate", rate),
            OptionItem::Detail => ("Detail", detail),
            OptionItem::Music => ("Music", on(music)),
            OptionItem::Effects => ("Effects", on(fx)),
            OptionItem::Device => ("Effects play on", device),
            OptionItem::Night => ("Night", on(night_flag())),
            OptionItem::Back => ("Back", ""),
        };
    }
    plate_rows(frame, 50, 66, 220, &rows[..n], s.cursor, s.tick);
    ui::text_centred(frame, W as i32 / 2, 188, "Up and Down choose, Left and Right change, Esc goes back", ui::GREY);
}

fn slots_screen(frame: &mut [u8], s: &Shell, kind: SlotKind) {
    ui::dim(frame, 0, 0, W as i32, H as i32, 7);
    heading(frame, if kind == SlotKind::Load { "Load game" } else { "Save game" }, 12);
    let (first, count) = slot_rows(kind);
    let mut labels: [Buf; SLOTS] = core::array::from_fn(|_| Buf::new());
    let mut values: [Buf; SLOTS] = core::array::from_fn(|_| Buf::new());
    for i in 0..count {
        let slot = first + i;
        if slot == 0 {
            let _ = write!(labels[i], "Autosave");
        } else if slot == save::QUICK {
            let _ = write!(labels[i], "Quick save");
        } else {
            let _ = write!(labels[i], "Slot {slot}");
        }
        match s.saves[slot] {
            None => {
                let _ = write!(values[i], "empty");
            }
            Some(info) => {
                let _ = write!(values[i], "{} cleared, {} pts, ", info.levels, info.score);
                save::date(info.stamp, &mut values[i]);
            }
        }
    }
    let mut rows: [(&str, &str); SLOTS + 1] = [("", ""); SLOTS + 1];
    for i in 0..count {
        rows[i] = (labels[i].as_str(), values[i].as_str());
    }
    rows[count] = ("Back", "");
    plate_rows(frame, 15, 40, 290, &rows[..=count], s.cursor, s.tick);
    ui::text_centred(frame, W as i32 / 2, 172, "Up and Down choose, Enter selects, Esc goes back", ui::GREY);
    match s.note {
        Some((1, slot, _)) => {
            let mut b = Buf::new();
            let _ = write!(b, "Saved to slot {slot}");
            ui::text_centred(frame, W as i32 / 2, 186, b.as_str(), ui::GREEN);
        }
        Some((_, _, _)) => ui::text_centred(frame, W as i32 / 2, 186, "Could not write the file", ui::RED),
        None => {}
    }
}

/// The boss's talk: a box at the foot of the screen, the speaker's name in their colour and the words typed out.
fn dialogue_box(frame: &mut [u8], d: Dialogue, tick: u32) {
    let lines = if d.id == 0 { BOSS_INTRO } else { BOSS_DEFEATED };
    let Some(&(who, text)) = lines.get(usize::from(d.line)) else {
        return;
    };
    let (x, y, w, h) = (6, H as i32 - 62, W as i32 - 12, 56);
    ui::dim(frame, x, y, w, h, 11);
    ui::outline(frame, x, y, w, h, ui::GREY);
    let colour = match who {
        "Ben" => ui::GREEN,
        "Tip" => ui::CYAN,
        _ => 13,
    };
    ui::text(frame, x + 8, y + 6, who, colour);
    ui::paragraph_upto(frame, x + 8, y + 6 + ui::LINE, w - 16, text, ui::WHITE, usize::from(d.typed));
    if usize::from(d.typed) >= text.chars().count() && (tick / 20) % 2 == 0 {
        ui::text(frame, x + w - 8 - ui::text_width("Enter"), y + h - ui::LINE - 3, "Enter", ui::CYAN);
    }
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
    toast_text(frame, TOASTS[usize::from(id).min(TOASTS.len() - 1)]);
}

fn toast_text(frame: &mut [u8], text: &str) {
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
        Card::End { score, lives } => {
            let _ = write!(title, "The end of Episode 1");
            let _ = write!(
                text,
                "Billy's home, the cocoa's back, and you finished with {score} snack points and {lives} {} to spare. Mildred will be back.",
                if lives == 1 { "life" } else { "lives" }
            );
            button = "Press Enter: back to the title";
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
