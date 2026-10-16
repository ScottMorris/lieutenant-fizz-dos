// The shell around the sim: what the web build's TypeScript does with the sim's events, for the DOS build.
//
// It keeps the map prompt, the toast, the floating caption words and the cards (level cleared, a fall, game over); the sim
// waits behind a card until Enter is pressed. All text is drawn over the finished frame (see `ui`).

use core::fmt::Write;
use core::ptr::addr_of_mut;

use lf_episode_1::state::{
    AMMO, BOSS_HP, HAS_USB, KEY_BLUE, KEY_GREEN, KEY_RED, LIVES, SCORE,
};
use lf_episode_1::{enter_map, events_clear, events_len, events_ptr, game_new, mode, state_get, step};

use crate::ui::{self, Buf};
use crate::ui_data::{
    BOSS_DEFEATED, BOSS_INTRO, CAPTIONS, LEVEL_BLURBS, LEVEL_CLEARED, LEVEL_NAMES, SIGNS, TOASTS,
};
use crate::video::{H, W};

extern "C" {
    /// The sound driver (C): plays the effect for a sim caption id.
    fn snd_caption(id: u32);
}

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

struct Shell {
    card: Option<Card>,
    card_age: u32,
    pending: Option<(Card, u32)>,
    /// The map prompt: its kind, id and extra number, as the sim sends them.
    prompt: Option<(u8, i32, i32)>,
    toast: Option<(u16, u32)>,
    captions: [Option<Caption>; 6],
    seen: [u32; CAPTIONS.len()],
    prev_held: u32,
    tick: u32,
}

static mut SHELL: Shell = Shell {
    card: None,
    card_age: 0,
    pending: None,
    prompt: None,
    toast: None,
    captions: [None; 6],
    seen: [0; CAPTIONS.len()],
    prev_held: 0,
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

/// Whether a card is up, so the sim is waiting.
pub fn card_up() -> bool {
    sh().card.is_some()
}

fn goto_map() {
    reset();
    crate::video::goto_map();
}

/// One tick: the sim steps (unless a card is up) and its events are read.
pub fn advance(held: u32) {
    let s = sh();
    s.tick += 1;
    let confirm = held & CONFIRM != 0 && s.prev_held & CONFIRM == 0;
    s.prev_held = held;

    if let Some(card) = s.card {
        s.card_age += 1;
        if confirm && s.card_age > CARD_ARM {
            if matches!(card, Card::GameOver(_)) {
                game_new();
            }
            goto_map();
        }
        return;
    }

    step(held);
    events(s);

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

/// Score, lives and Fizz along the top, with the next life, the keys held and the boss's health below.
fn hud(frame: &mut [u8]) {
    ui::dim(frame, 0, 0, W as i32, 20, 7);
    let score = state_get(SCORE) as i32;
    let lives = (state_get(LIVES) as i32).max(0);
    let ammo = state_get(AMMO) as i32;
    let mut buf = Buf::new();
    let mut x = 5;
    for (label, value, colour) in [
        ("Score", score, ui::YELLOW),
        ("Lives", lives, ui::GREEN),
        ("Fizz", ammo, ui::CYAN),
    ] {
        x = ui::text(frame, x, 2, label, ui::GREY) + 3;
        let shown = if label == "Score" {
            *&mut buf = Buf::new();
            let _ = write!(buf, "{value:06}");
            buf.as_str()
        } else {
            number(&mut buf, value)
        };
        x = ui::text(frame, x, 2, shown, colour) + 12;
    }
    // The next extra life comes at every hundred snack points.
    let mut next = Buf::new();
    let _ = write!(next, "Next life at {}", (score / 100 + 1) * 100);
    ui::text(frame, 5, 11, next.as_str(), ui::GREY);
    // Keys held, as coloured gumdrops and the gold USB.
    let mut kx = 120;
    for (held, colour) in [(KEY_RED, ui::RED), (KEY_BLUE, ui::BLUE), (KEY_GREEN, ui::GREEN)] {
        if state_get(held) != 0.0 {
            ui::fill(frame, kx, 12, 7, 6, colour);
            ui::outline(frame, kx - 1, 11, 9, 8, ui::BLACK);
            kx += 12;
        }
    }
    if state_get(HAS_USB) != 0.0 {
        ui::text(frame, kx, 11, "USB", ui::YELLOW);
    }
    // The boss's health: three pips.
    let hp = state_get(BOSS_HP) as i32;
    if mode() == 3 && hp > 0 {
        let mut pips = Buf::new();
        for i in 0..3 {
            let _ = pips.write_str(if i < hp { "\u{25CF}" } else { "\u{25CB}" });
        }
        let w = ui::text_width("Cocoa Colossus ") + ui::text_width(pips.as_str());
        let bx = W as i32 - w - 5;
        let end = ui::text(frame, bx, 2, "Cocoa Colossus ", ui::RED);
        ui::text(frame, end, 2, pips.as_str(), ui::YELLOW);
    }
}

fn toast(frame: &mut [u8], id: u16) {
    let text = TOASTS[usize::from(id).min(TOASTS.len() - 1)];
    let mut lines = [(0, 0); 4];
    let n = ui::wrap(text, 290, &mut lines);
    let h = n as i32 * ui::LINE + 6;
    ui::dim(frame, 8, 24, W as i32 - 16, h, 9);
    ui::outline(frame, 8, 24, W as i32 - 16, h, ui::DARK);
    ui::paragraph(frame, 14, 27, 292, text, ui::WHITE);
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
    let n = ui::wrap(text.as_str(), 292, &mut lines) as i32;
    let h = 8 + ui::LINE + n * ui::LINE + if action.is_some() { ui::LINE } else { 0 };
    let y = H as i32 - 4 - h;
    ui::dim(frame, 6, y, W as i32 - 12, h, 9);
    ui::outline(frame, 6, y, W as i32 - 12, h, ui::DARK);
    ui::text(frame, 12, y + 4, title, ui::YELLOW);
    ui::paragraph(frame, 12, y + 4 + ui::LINE, 296, text.as_str(), ui::WHITE);
    if let Some(a) = action {
        let mut label = Buf::new();
        let _ = write!(label, "Press Enter, Jump or Fire to {}", a.to_ascii_lowercase());
        ui::text(frame, 12, y + 4 + ui::LINE * (n + 1), label.as_str(), ui::CYAN);
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
    let h = 12 + ui::LINE * 2 + n * ui::LINE + 6;
    let (w, x, y) = (252, (W as i32 - 252) / 2, (H as i32 - h) / 2);
    ui::dim(frame, x, y, w, h, 14);
    ui::fill(frame, x, y, w, 1, ui::YELLOW);
    ui::outline(frame, x, y, w, h, ui::DARK);
    ui::text_centred(frame, W as i32 / 2, y + 6, title.as_str(), ui::YELLOW);
    ui::paragraph(frame, x + 10, y + 6 + ui::LINE + 4, 232, text.as_str(), ui::WHITE);
    ui::text_centred(frame, W as i32 / 2, y + h - ui::LINE - 3, button, ui::CYAN);
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
