/* Native check of the key map.
 *
 * (c) Copyright 2026 Liminal HQ, Scott Morris
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

/* Native check of the key map: gcc -o keymap_test keymap_test.c keymap.c mouse.c && ./keymap_test */
#include <stdio.h>
#include <string.h>
#include "keymap.h"
#include "mouse.h"
static int failed;
static void expect(const char *what, unsigned got, unsigned want) {
  if (got != want) { printf("FAIL %s: got %u want %u\n", what, got, want); failed = 1; }
}

static volatile unsigned char k[128];
static void clear(void) { memset((void *)k, 0, sizeof k); }

/* Every scheme's keys as jump, pogo and fire, and a second fire key (0 for none). The Mouse scheme (3) has only a jump key, so it is checked on its own below. */
static const struct { int scheme; const char *name; int jump, pogo, fire, fire2; } schemes[] = {
  {0, "Keen-style", SC_CTRL, SC_ALT, SC_SPACE, 0},
  {1, "Modern", SC_Z, SC_X, SC_C, SC_SPACE},
  {2, "WASD", SC_SPACE, SC_K, SC_J, 0},
  {4, "Left hand", SC_SPACE, SC_Q, SC_E, 0},
  {5, "Right hand", SC_COMMA, SC_PERIOD, SC_SLASH, 0},
};
#define NSCHEMES ((int)(sizeof schemes / sizeof schemes[0]))

/* One held key must give exactly these bits in this scheme. */
static void one_key(int n, const char *label, int key, unsigned want) {
  char what[96];
  clear();
  k[key] = 1;
  sprintf(what, "%s: %s", schemes[n].name, label);
  expect(what, keymap_bits(k, schemes[n].scheme), want);
}

static void check_scheme(int n) {
  char what[96];
  int s = schemes[n].scheme, m, a, j;
  const int keys[4] = {schemes[n].jump, schemes[n].pogo, schemes[n].fire, schemes[n].fire2};
  const unsigned bits[4] = {IN_JUMP, IN_POGO, IN_FIRE, IN_FIRE};
  static const int reserved[] = {SC_ESC, SC_ENTER, SC_F5, SC_F9, SC_P, 0x21 /* F */, SC_R, SC_N, SC_M, SC_PGUP, SC_PGDN, SC_BACKSPACE};
  for (a = 0; a < 3 + (keys[3] != 0); a++) {
    one_key(n, "its action key", keys[a], a == 3 ? IN_FIRE : bits[a]);
    expect("is an action key", (unsigned)keymap_is_action(s, keys[a]), 1);
  }
  clear();
  k[keys[0]] = k[keys[1]] = k[keys[2]] = 1;
  sprintf(what, "%s: all three action keys", schemes[n].name);
  expect(what, keymap_bits(k, s), IN_JUMP | IN_POGO | IN_FIRE);
  /* The other schemes' keys do nothing, except one this scheme uses as well. */
  for (m = 0; m < NSCHEMES; m++) {
    const int other[4] = {schemes[m].jump, schemes[m].pogo, schemes[m].fire, schemes[m].fire2};
    if (m == n) continue;
    for (a = 0; a < 4; a++) {
      int shared = other[a] == 0;
      for (j = 0; j < 4; j++) if (keys[j] == other[a]) shared = 1;
      if (shared) continue;
      sprintf(what, "another scheme's key %02X", other[a]);
      one_key(n, what, other[a], 0);
      expect("not an action key", (unsigned)keymap_is_action(s, other[a]), 0);
    }
  }
  /* Movement, menu and quick keys are the same in every scheme. */
  one_key(n, "left arrow", SC_LEFT, IN_LEFT);
  one_key(n, "A", SC_A, IN_LEFT);
  one_key(n, "right arrow", SC_RIGHT, IN_RIGHT);
  one_key(n, "D", SC_D, IN_RIGHT);
  one_key(n, "up arrow", SC_UP, IN_UP);
  one_key(n, "W", SC_W, IN_UP);
  one_key(n, "down arrow", SC_DOWN, IN_DOWN);
  one_key(n, "S", SC_S, IN_DOWN);
  one_key(n, "Esc", SC_ESC, IN_MENU);
  one_key(n, "Enter", SC_ENTER, IN_CONFIRM);
  one_key(n, "F5", SC_F5, IN_QSAVE);
  one_key(n, "F9", SC_F9, IN_QLOAD);
  one_key(n, "Page Up", SC_PGUP, 0);
  /* No scheme takes Esc, Enter, F5, F9, P, F or the keys the main loop keeps for itself. */
  for (a = 0; a < (int)(sizeof reserved / sizeof reserved[0]); a++) {
    sprintf(what, "%s: reserved key %02X is not an action", schemes[n].name, reserved[a]);
    expect(what, (unsigned)keymap_is_action(s, reserved[a]), 0);
  }
}

int main(void) {
  int i, s;
  clear();
  expect("nothing held", keymap_bits(k, KEY_SCHEME_DEFAULT), 0);
  expect("the default is Modern", KEY_SCHEME_DEFAULT, 1);
  for (i = 0; i < NSCHEMES; i++) check_scheme(i);
  /* A mixed hold, as in play. */
  clear(); k[SC_D] = 1; k[SC_Z] = 1; k[SC_C] = 1;
  expect("Modern: D, Z and C", keymap_bits(k, 1), IN_RIGHT | IN_JUMP | IN_FIRE);
  clear(); k[SC_ALT] = 1; k[SC_UP] = 1; k[SC_LEFT] = 1;
  expect("Keen-style: Alt up left", keymap_bits(k, 0), IN_POGO | IN_UP | IN_LEFT);
  /* Modern fires with C or Space; every other scheme keeps its own Space meaning. */
  clear(); k[SC_SPACE] = 1;
  expect("Modern: Space fires", keymap_bits(k, 1), IN_FIRE);
  expect("Modern: Space is an action key", (unsigned)keymap_is_action(1, SC_SPACE), 1);
  clear(); k[SC_C] = 1;
  expect("Modern: C still fires", keymap_bits(k, 1), IN_FIRE);
  clear(); k[SC_SPACE] = 1; k[SC_C] = 1;
  expect("Modern: Space and C give one fire", keymap_bits(k, 1), IN_FIRE);
  clear(); k[SC_SPACE] = 1;
  expect("WASD: Space jumps", keymap_bits(k, 2), IN_JUMP);
  expect("Left hand: Space jumps", keymap_bits(k, 4), IN_JUMP);
  expect("Keen-style: Space fires", keymap_bits(k, 0), IN_FIRE);
  expect("Right hand: Space does nothing", keymap_bits(k, 5), 0);
  expect("Right hand: Space is not an action key", (unsigned)keymap_is_action(5, SC_SPACE), 0);
  expect("no scheme treats key 0 as an action", (unsigned)keymap_is_action(1, 0) + keymap_is_action(0, 0), 0);
  /* Without a mouse (the state at start-up before one is found) the Mouse scheme is not offered: its number and numbers out of range read as
     Modern, and Options never steps to it. */
  keymap_set_mouse(0);
  expect("no mouse: Mouse reads as Modern", (unsigned)keymap_scheme(KEY_SCHEME_MOUSE), 1);
  expect("negative reads as Modern", (unsigned)keymap_scheme(-1), 1);
  expect("past the last reads as Modern", (unsigned)keymap_scheme(KEY_SCHEME_COUNT), 1);
  clear(); k[SC_Z] = 1;
  expect("no mouse: scheme 3 plays as Modern", keymap_bits(k, KEY_SCHEME_MOUSE), IN_JUMP);
  clear(); k[SC_SPACE] = 1;
  expect("no mouse: scheme 3 has Modern's Space fire", keymap_bits(k, KEY_SCHEME_MOUSE), IN_FIRE);
  {
    static const int order[] = {0, 1, 2, 4, 5, 0};
    for (i = 0; i < 5; i++) expect("no mouse: the order Options steps through", (unsigned)keymap_next(order[i]), (unsigned)order[i + 1]);
    for (s = 0; s < KEY_SCHEME_COUNT; s++) expect("no mouse: next is never Mouse", keymap_next(s) != KEY_SCHEME_MOUSE, 1);
    expect("no mouse: a saved Mouse steps on from Modern", (unsigned)keymap_next(KEY_SCHEME_MOUSE), 2);
  }
  /* With a mouse the Mouse scheme is offered and Options steps 0, 1, 2, 3, 4, 5 and round. */
  keymap_set_mouse(1);
  expect("mouse: Mouse is offered", (unsigned)keymap_scheme(KEY_SCHEME_MOUSE), KEY_SCHEME_MOUSE);
  expect("mouse: out of range is still Modern", (unsigned)keymap_scheme(KEY_SCHEME_COUNT), 1);
  {
    static const int order[] = {0, 1, 2, 3, 4, 5, 0};
    for (i = 0; i < 6; i++) expect("mouse: the order Options steps through", (unsigned)keymap_next(order[i]), (unsigned)order[i + 1]);
  }
  /* The Mouse scheme's keys: Space jumps, WASD and the arrows move, and no key pogoes or fires (the buttons do). */
  clear(); k[SC_SPACE] = 1;
  expect("Mouse: Space jumps", keymap_bits(k, KEY_SCHEME_MOUSE), IN_JUMP);
  expect("Mouse: Space is an action key", (unsigned)keymap_is_action(KEY_SCHEME_MOUSE, SC_SPACE), 1);
  clear(); k[SC_D] = 1; k[SC_UP] = 1;
  expect("Mouse: D and up move", keymap_bits(k, KEY_SCHEME_MOUSE), IN_RIGHT | IN_UP);
  {
    static const int others[] = {SC_Z, SC_X, SC_C, SC_K, SC_J, SC_Q, SC_E, SC_CTRL, SC_ALT, SC_COMMA, SC_PERIOD, SC_SLASH, 0};
    for (i = 0; others[i]; i++) {
      clear(); k[others[i]] = 1;
      expect("Mouse: no other key pogoes or fires or jumps", keymap_bits(k, KEY_SCHEME_MOUSE), 0);
      expect("Mouse: no other action key", (unsigned)keymap_is_action(KEY_SCHEME_MOUSE, others[i]), 0);
    }
  }
  clear(); k[0] = 1;
  expect("Mouse: key 0 does nothing", keymap_bits(k, KEY_SCHEME_MOUSE), 0);
  /* The buttons as bits: left is Fizz, right is the pogo, held or only clicked. */
  expect("no buttons", mouse_bits(0, 0, 0), 0);
  expect("left held", mouse_bits(1, 0, 0), IN_FIRE);
  expect("right held", mouse_bits(2, 0, 0), IN_POGO);
  expect("both held", mouse_bits(3, 0, 0), IN_FIRE | IN_POGO);
  expect("left clicked within a tick", mouse_bits(0, 1, 0), IN_FIRE);
  expect("right clicked within a tick", mouse_bits(0, 0, 1), IN_POGO);
  expect("several clicks give one bit", mouse_bits(0, 3, 2), IN_FIRE | IN_POGO);
  expect("left held and right clicked", mouse_bits(1, 0, 1), IN_FIRE | IN_POGO);
  expect("a middle button (bit 2) does nothing", mouse_bits(4, 0, 0), 0);
  /* Without a driver (here) nothing is read, and the module says no mouse until it has found one. */
  expect("native: no mouse found", (unsigned)mouse_init(), 0);
  expect("native: mouse_present is 0", (unsigned)mouse_present(), 0);
  expect("native: no bits without a mouse", mouse_tick_bits(), 0);
  /* A saved Mouse choice on a machine without a mouse plays as Modern and is not changed: the number the caller keeps is still 3. */
  {
    int saved = KEY_SCHEME_MOUSE;
    keymap_set_mouse(0);
    expect("saved Mouse, no mouse: plays as Modern", (unsigned)keymap_scheme(saved), 1);
    expect("saved Mouse, no mouse: the saved number is untouched", (unsigned)saved, KEY_SCHEME_MOUSE);
    keymap_set_mouse(1);
    expect("saved Mouse, a mouse again: plays as Mouse", (unsigned)keymap_scheme(saved), KEY_SCHEME_MOUSE);
  }
  if (!failed) printf("keymap ok\n");
  return failed;
}
