/* Interface to the IRQ 1 keyboard handler.
 *
 * (c) Copyright 2026 Liminal HQ, Scott Morris
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

#ifndef KBD_H
#define KBD_H
int kbd_install(void);   /* hook IRQ 1; 0 on success */
void kbd_remove(void);   /* give the keyboard back to DOS (also run at exit) */
void kbd_set_scheme(int scheme); /* the key scheme to read the keys by (see keymap.h); a number this build does not offer is Modern */
int kbd_scheme(void);  /* the scheme chosen, as given: what FIZZ.CFG keeps (never the retired Mouse number 3: it is stored as WASD) */
int kbd_effective_scheme(void); /* the scheme played and shown: the chosen one, (Modern for a number out of range) */
unsigned kbd_held(void); /* the game's input bits right now (the mouse buttons as of the last tick, in WASD or Mouse jump with a mouse) */
int kbd_down(int scancode);
/* The game bits for one tick: what is held, plus any key pressed and released since the last tick (so a quick tap is never lost), and IN_ANY for a key of any kind pressed since the last call. In WASD or Mouse jump with a mouse the mouse's held buttons and clicks since the last tick (INT 33h) are added; in no other scheme, or without a mouse, is it read. */
unsigned kbd_tick_bits(void);
/* 1 once for each press of the key since the last call. */
int kbd_pressed(int scancode);
/* The scancode of the oldest key press not yet taken, in the order they came (a held key's repeats are not presses), or -1 when there is none. For key sequences; it does not touch what the other calls return. */
int kbd_raw_pop(void);
#endif
