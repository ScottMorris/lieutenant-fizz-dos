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

/* Every scheme's keys as jump, pogo and fire, and a second fire key (0 for none). Number 3 is retired and checked on its own below. */
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
  /* The retired Mouse number (3, from an earlier build) and numbers out of range: 3 plays as WASD, the rest as Modern. */
  expect("scheme 3 reads as WASD", (unsigned)keymap_scheme(3), KEY_SCHEME_WASD);
  expect("negative reads as Modern", (unsigned)keymap_scheme(-1), 1);
  expect("past the last reads as Modern", (unsigned)keymap_scheme(KEY_SCHEME_COUNT), 1);
  for (s = 0; s < KEY_SCHEME_COUNT; s++)
    if (s != 3) expect("a live scheme reads as itself", (unsigned)keymap_scheme(s), (unsigned)s);
  clear(); k[SC_SPACE] = 1;
  expect("scheme 3: Space jumps as in WASD", keymap_bits(k, 3), IN_JUMP);
  clear(); k[SC_K] = 1;
  expect("scheme 3: K pogoes as in WASD", keymap_bits(k, 3), IN_POGO);
  clear(); k[SC_J] = 1;
  expect("scheme 3: J fires as in WASD", keymap_bits(k, 3), IN_FIRE);
  expect("scheme 3: K is an action key", (unsigned)keymap_is_action(3, SC_K), 1);
  clear(); k[SC_Z] = 1; k[SC_X] = 1; k[SC_C] = 1;
  expect("scheme 3: Modern's keys do nothing", keymap_bits(k, 3), 0);
  clear(); k[SC_SPACE] = k[SC_K] = k[SC_J] = 1;
  expect("scheme 3 gives the same bits as 2", keymap_bits(k, 3), keymap_bits(k, 2));
  /* Stepping forwards visits 0, 1, 2, 4, 5 and wraps; backwards visits 5, 4, 2, 1, 0 and wraps. Number 3 never appears. */
  {
    static const int order[] = {0, 1, 2, 4, 5};
    int at = 0;
    for (i = 0; i < 12; i++) {
      at = keymap_step(at, 1);
      expect("forwards: the order Options steps through", (unsigned)at, (unsigned)order[(i + 1) % 5]);
      expect("forwards: never the retired number", (unsigned)(at != KEY_SCHEME_RETIRED), 1);
    }
    at = 0;
    for (i = 0; i < 12; i++) {
      at = keymap_step(at, -1);
      expect("backwards: the reverse order", (unsigned)at, (unsigned)order[((-(i + 1)) % 5 + 5) % 5]);
      expect("backwards: never the retired number", (unsigned)(at != KEY_SCHEME_RETIRED), 1);
    }
    expect("forwards from 5 wraps to 0", (unsigned)keymap_step(5, 1), 0);
    expect("backwards from 0 wraps to 5", (unsigned)keymap_step(0, -1), 5);
    expect("backwards from 5 is 4", (unsigned)keymap_step(5, -1), 4);
    expect("backwards from 2 is 1", (unsigned)keymap_step(2, -1), 1);
    expect("forwards from 2 is 4", (unsigned)keymap_step(2, 1), 4);
    expect("forwards from a saved 3 is 4 (from WASD)", (unsigned)keymap_step(3, 1), 4);
    expect("backwards from a saved 3 is 1 (from WASD)", (unsigned)keymap_step(3, -1), 1);
    expect("an out-of-range number steps from Modern", (unsigned)keymap_step(9, 1), 2);
  }
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
  if (!failed) printf("keymap ok\n");
  return failed;
}
