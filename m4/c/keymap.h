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
/* Scancodes (set 1, the low 7 bits of what port 0x60 gives; the 0xE0 prefix is ignored, so the grey arrows and the numpad arrows are the same). */
enum {
  SC_ESC = 0x01, SC_ENTER = 0x1C, SC_W = 0x11, SC_R = 0x13, SC_CTRL = 0x1D, SC_A = 0x1E, SC_S = 0x1F, SC_D = 0x20,
  SC_Z = 0x2C, SC_X = 0x2D, SC_C = 0x2E, SC_ALT = 0x38, SC_SPACE = 0x39, SC_P = 0x19, SC_O = 0x18, SC_N = 0x31, SC_M = 0x32, SC_BACKSPACE = 0x0E, SC_E = 0x12,
  SC_UP = 0x48, SC_PGUP = 0x49, SC_LEFT = 0x4B, SC_RIGHT = 0x4D, SC_DOWN = 0x50, SC_PGDN = 0x51
};
/* The held bits for a table of 128 key states (non-zero is down): arrows or WASD move; Ctrl or Z jump; Alt or X pogo; Space or C fire. */
unsigned keymap_bits(const volatile unsigned char *keys);
#endif
