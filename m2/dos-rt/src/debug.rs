// The hidden Debug menu's key sequence: which keys open it, how long the player has between them, and the matcher that watches for it.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// The sequence is the main game's: `2`, then Enter, with at most 1.5 seconds between them. Change it here and nowhere else. The keys are
// keyboard scancodes (set 1, as `m4/c/keymap.h` has them), which the C side hands over as raw presses (`kbd_raw_pop`).

/// The scancode of the main row's `2`. The keypad's `2` sends the Down arrow's code, so it cannot be told apart and does not count.
pub const SC_2: u8 = 0x03;
/// The scancode of Enter (the keypad's Enter sends the same code).
pub const SC_ENTER: u8 = 0x1C;

/// The keys that open the Debug menu, in order.
pub const DEBUG_SEQUENCE: [u8; 2] = [SC_2, SC_ENTER];
/// The longest gap, in milliseconds, between one key of the sequence and the next.
pub const DEBUG_SEQUENCE_MS: u32 = 1500;

/// The Debug menu's Mode row, flipped: Normal for Expert and Expert for Normal.
pub fn toggled(normal: bool) -> bool {
    !normal
}

/// What the Mode row shows for a mode.
pub fn mode_name(normal: bool) -> &'static str {
    if normal {
        "Normal"
    } else {
        "Expert"
    }
}

/// Whether a scancode is a Shift, Ctrl or Alt key. Pressing one neither advances the sequence nor drops its progress.
pub fn is_modifier(code: u8) -> bool {
    matches!(code, 0x2A | 0x36 | 0x1D | 0x38)
}

/// Watches key presses for a sequence. It holds no clock of its own: the caller says what time each press came.
pub struct SequenceMatcher {
    steps: &'static [u8],
    timeout_ms: u32,
    at: usize,
    last: u32,
}

impl SequenceMatcher {
    /// A matcher for `steps`, where a step that comes more than `timeout_ms` after the one before starts over.
    pub const fn new(steps: &'static [u8], timeout_ms: u32) -> Self {
        SequenceMatcher { steps, timeout_ms, at: 0, last: 0 }
    }

    /// Feeds one key press at `now` (milliseconds); true when this press completes the sequence. A key that is not the next step drops the
    /// progress (and counts as a fresh start when it is the first step).
    pub fn press(&mut self, code: u8, now: u32) -> bool {
        if self.at > 0 && now.wrapping_sub(self.last) > self.timeout_ms {
            self.at = 0;
        }
        if self.steps.get(self.at) == Some(&code) {
            self.at += 1;
            self.last = now;
            if self.at == self.steps.len() {
                self.at = 0;
                return true;
            }
            return false;
        }
        self.at = usize::from(self.steps.first() == Some(&code));
        self.last = now;
        false
    }

    /// Forgets any progress.
    pub fn reset(&mut self) {
        self.at = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matcher() -> SequenceMatcher {
        SequenceMatcher::new(&DEBUG_SEQUENCE, DEBUG_SEQUENCE_MS)
    }

    #[test]
    fn the_sequence_opens_the_menu() {
        let mut m = matcher();
        assert!(!m.press(SC_2, 0));
        assert!(m.press(SC_ENTER, 100));
    }

    #[test]
    fn it_is_two_then_enter_in_that_order() {
        let mut m = matcher();
        assert!(!m.press(SC_ENTER, 0));
        assert!(!m.press(SC_2, 10));
        assert!(!m.press(SC_2, 20));
        assert!(m.press(SC_ENTER, 30));
    }

    #[test]
    fn enter_alone_does_nothing() {
        let mut m = matcher();
        assert!(!m.press(SC_ENTER, 0));
        assert!(!m.press(SC_ENTER, 100));
    }

    #[test]
    fn a_lone_two_does_nothing_however_often_it_comes() {
        let mut m = matcher();
        for i in 0..5 {
            assert!(!m.press(SC_2, i * 100));
        }
    }

    #[test]
    fn a_wrong_key_between_resets() {
        let mut m = matcher();
        assert!(!m.press(SC_2, 0));
        assert!(!m.press(0x1E, 50));
        assert!(!m.press(SC_ENTER, 100));
        // And the sequence still works afterwards.
        assert!(!m.press(SC_2, 200));
        assert!(m.press(SC_ENTER, 300));
    }

    #[test]
    fn the_gap_may_be_the_full_timeout_and_no_more() {
        let mut m = matcher();
        assert!(!m.press(SC_2, 1000));
        assert!(m.press(SC_ENTER, 1000 + DEBUG_SEQUENCE_MS));
        assert!(!m.press(SC_2, 5000));
        assert!(!m.press(SC_ENTER, 5000 + DEBUG_SEQUENCE_MS + 1));
    }

    #[test]
    fn a_late_two_starts_a_fresh_sequence() {
        let mut m = matcher();
        assert!(!m.press(SC_2, 0));
        assert!(!m.press(SC_2, 5000));
        assert!(m.press(SC_ENTER, 5100));
    }

    #[test]
    fn reset_drops_the_progress() {
        let mut m = matcher();
        assert!(!m.press(SC_2, 0));
        m.reset();
        assert!(!m.press(SC_ENTER, 10));
    }

    #[test]
    fn the_clock_may_wrap() {
        let mut m = matcher();
        assert!(!m.press(SC_2, u32::MAX - 10));
        assert!(m.press(SC_ENTER, 20));
    }

    #[test]
    fn the_mode_row_flips_between_the_two_modes() {
        assert!(toggled(false));
        assert!(!toggled(true));
        assert!(!toggled(toggled(false)));
        assert_eq!(mode_name(false), "Expert");
        assert_eq!(mode_name(true), "Normal");
    }

    #[test]
    fn modifiers_are_told_apart() {
        for code in [0x2A, 0x36, 0x1D, 0x38] {
            assert!(is_modifier(code));
        }
        assert!(!is_modifier(SC_2));
        assert!(!is_modifier(SC_ENTER));
    }
}
