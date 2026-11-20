/* Native check of the key map.
 *
 * (c) Copyright 2026 Liminal HQ, Scott Morris
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

/* Native check of the key map: gcc -o keymap_test keymap_test.c keymap.c && ./keymap_test */
#include <stdio.h>
#include <string.h>
#include "keymap.h"
static int failed;
static void expect(const char *what, unsigned got, unsigned want) {
  if (got != want) { printf("FAIL %s: got %u want %u\n", what, got, want); failed = 1; }
}

static volatile unsigned char k[128];
static void clear(void) { memset((void *)k, 0, sizeof k); }

/* Every scheme's keys as jump, pogo and fire. The Mouse scheme (3) has none here, so it is not listed. */
static const struct { int scheme; const char *name; int jump, pogo, fire; } schemes[] = {
  {0, "Keen-style", SC_CTRL, SC_ALT, SC_SPACE},
  {1, "Modern", SC_Z, SC_X, SC_C},
  {2, "WASD", SC_SPACE, SC_K, SC_J},
  {4, "Left hand", SC_SPACE, SC_Q, SC_E},
  {5, "Right hand", SC_COMMA, SC_PERIOD, SC_SLASH},
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
  const int keys[3] = {schemes[n].jump, schemes[n].pogo, schemes[n].fire};
  const unsigned bits[3] = {IN_JUMP, IN_POGO, IN_FIRE};
  static const int reserved[] = {SC_ESC, SC_ENTER, SC_F5, SC_F9, SC_P, 0x21 /* F */, SC_R, SC_N, SC_M, SC_PGUP, SC_PGDN, SC_BACKSPACE};
  for (a = 0; a < 3; a++) {
    one_key(n, "its action key", keys[a], bits[a]);
    expect("is an action key", (unsigned)keymap_is_action(s, keys[a]), 1);
  }
  clear();
  k[keys[0]] = k[keys[1]] = k[keys[2]] = 1;
  sprintf(what, "%s: all three action keys", schemes[n].name);
  expect(what, keymap_bits(k, s), IN_JUMP | IN_POGO | IN_FIRE);
  /* The other schemes' keys do nothing, except one this scheme uses as well. */
  for (m = 0; m < NSCHEMES; m++) {
    const int other[3] = {schemes[m].jump, schemes[m].pogo, schemes[m].fire};
    if (m == n) continue;
    for (a = 0; a < 3; a++) {
      int shared = 0;
      for (j = 0; j < 3; j++) if (keys[j] == other[a]) shared = 1;
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
  /* The Mouse scheme is not offered: its number, and numbers out of range, read as Modern. */
  expect("Mouse reads as Modern", (unsigned)keymap_scheme(KEY_SCHEME_MOUSE), 1);
  expect("negative reads as Modern", (unsigned)keymap_scheme(-1), 1);
  expect("past the last reads as Modern", (unsigned)keymap_scheme(KEY_SCHEME_COUNT), 1);
  clear(); k[SC_Z] = 1;
  expect("scheme 3 plays as Modern", keymap_bits(k, KEY_SCHEME_MOUSE), IN_JUMP);
  /* Options steps 0, 1, 2, 4, 5 and round again; a step is never the Mouse scheme. */
  {
    static const int order[] = {0, 1, 2, 4, 5, 0};
    for (i = 0; i < 5; i++) expect("the order Options steps through", (unsigned)keymap_next(order[i]), (unsigned)order[i + 1]);
    for (s = 0; s < KEY_SCHEME_COUNT; s++) expect("next is never Mouse", keymap_next(s) != KEY_SCHEME_MOUSE, 1);
  }
  if (!failed) printf("keymap ok\n");
  return failed;
}
