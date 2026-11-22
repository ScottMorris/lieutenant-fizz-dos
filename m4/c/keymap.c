/* Maps keyboard scancodes to the game's input bits.
 *
 * (c) Copyright 2026 Liminal HQ, Scott Morris
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

#include "keymap.h"

/* Jump, pogo and fire for each scheme, by number. The Mouse scheme (3) has no keys for the actions, and is not offered here. */
static const unsigned char action_keys[KEY_SCHEME_COUNT][3] = {
  {SC_CTRL, SC_ALT, SC_SPACE},   /* 0 Keen-style */
  {SC_Z, SC_X, SC_C},            /* 1 Modern */
  {SC_SPACE, SC_K, SC_J},        /* 2 WASD */
  {0, 0, 0},                     /* 3 Mouse */
  {SC_SPACE, SC_Q, SC_E},        /* 4 Left hand */
  {SC_COMMA, SC_PERIOD, SC_SLASH} /* 5 Right hand */
};

int keymap_scheme(int scheme) {
  return scheme < 0 || scheme >= KEY_SCHEME_COUNT || scheme == KEY_SCHEME_MOUSE ? KEY_SCHEME_DEFAULT : scheme;
}

int keymap_next(int scheme) {
  int next = (keymap_scheme(scheme) + 1) % KEY_SCHEME_COUNT;
  return next == KEY_SCHEME_MOUSE ? next + 1 : next;
}

int keymap_is_action(int scheme, int sc) {
  const unsigned char *a = action_keys[keymap_scheme(scheme)];
  return sc == a[0] || sc == a[1] || sc == a[2];
}

unsigned keymap_bits(const volatile unsigned char *k, int scheme) {
  const unsigned char *a = action_keys[keymap_scheme(scheme)];
  unsigned b = 0;
  if (k[SC_LEFT] || k[SC_A]) b |= IN_LEFT;
  if (k[SC_RIGHT] || k[SC_D]) b |= IN_RIGHT;
  if (k[SC_UP] || k[SC_W]) b |= IN_UP;
  if (k[SC_DOWN] || k[SC_S]) b |= IN_DOWN;
  if (k[a[0]]) b |= IN_JUMP;
  if (k[a[1]]) b |= IN_POGO;
  if (k[a[2]]) b |= IN_FIRE;
  if (k[SC_ENTER]) b |= IN_CONFIRM;
  if (k[SC_ESC]) b |= IN_MENU;
  if (k[SC_F5]) b |= IN_QSAVE;
  if (k[SC_F9]) b |= IN_QLOAD;
  return b;
}
