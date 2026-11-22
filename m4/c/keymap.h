/* Scancodes, input bits and the key map's interface.
 *
 * (c) Copyright 2026 Liminal HQ, Scott Morris
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

#ifndef KEYMAP_H
#define KEYMAP_H
/* Game input bits, as the sim reads them. */
#define IN_LEFT 1
#define IN_RIGHT 2
#define IN_UP 4
#define IN_DOWN 8
#define IN_JUMP 16
#define IN_POGO 32
#define IN_FIRE 64
#define IN_CONFIRM 128
#define IN_MENU 256 /* Escape: pauses, and goes back in a menu */
#define IN_QSAVE 512 /* F5: quick-save */
#define IN_QLOAD 1024 /* F9: quick-load */
#define IN_ANY 2048 /* any key pressed since the last tick (a tap, never a held key): what skips the opening titles */
/* Scancodes (set 1, the low 7 bits of what port 0x60 gives; the 0xE0 prefix is ignored, so the grey arrows and the numpad arrows are the same). */
enum {
  SC_ESC = 0x01, SC_Q = 0x10, SC_J = 0x24, SC_K = 0x25, SC_COMMA = 0x33, SC_PERIOD = 0x34, SC_SLASH = 0x35, SC_ENTER = 0x1C, SC_W = 0x11, SC_R = 0x13, SC_CTRL = 0x1D, SC_A = 0x1E, SC_S = 0x1F, SC_D = 0x20,
  SC_Z = 0x2C, SC_X = 0x2D, SC_C = 0x2E, SC_ALT = 0x38, SC_SPACE = 0x39, SC_P = 0x19, SC_O = 0x18, SC_N = 0x31, SC_M = 0x32, SC_BACKSPACE = 0x0E, SC_E = 0x12,
  SC_F5 = 0x3F, SC_F9 = 0x43,
  SC_UP = 0x48, SC_PGUP = 0x49, SC_LEFT = 0x4B, SC_RIGHT = 0x4D, SC_DOWN = 0x50, SC_PGDN = 0x51
};
/* The key schemes. The number is what FIZZ.CFG keeps and what the web game's Options keep, so a scheme means the same in both:
   0 Keen-style (Ctrl, Alt, Space), 1 Modern (Z, X, and C or Space), 2 WASD (Space, K, J), 3 Mouse (web only; not offered here), 4 Left hand (Space,
   Q, E), 5 Right hand (comma, period, slash), as jump, pogo and fire. */
#define KEY_SCHEME_COUNT 6
#define KEY_SCHEME_DEFAULT 1
#define KEY_SCHEME_MOUSE 3
/* The scheme to use for a saved or given number: itself when it is one this build offers, else Modern. */
int keymap_scheme(int scheme);
/* The scheme after this one in the order Options steps through, skipping the Mouse scheme. */
int keymap_next(int scheme);
/* Whether the scancode is one of the scheme's jump, pogo or fire keys (Modern has two fire keys). */
int keymap_is_action(int scheme, int scancode);
/* The held bits for a table of 128 key states (non-zero is down): the arrows or WASD move in every scheme; the scheme's own keys jump,
   pogo and fire, and no other scheme's do. Enter, Esc, F5 and F9 are the same in every scheme. */
unsigned keymap_bits(const volatile unsigned char *keys, int scheme);
#endif
