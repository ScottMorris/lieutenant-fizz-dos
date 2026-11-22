/* Maps keyboard scancodes to the game's input bits.
 *
 * (c) Copyright 2026 Liminal HQ, Scott Morris
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

#include "keymap.h"

/* Jump, pogo, fire and a second fire key (0 for none) for each scheme, by number. Number 3 was the Mouse scheme and is retired: it has no row of its own, and keymap_scheme() gives WASD for it. The mouse's buttons are WASD's extra pogo and fire (mouse.c). */
static const unsigned char action_keys[KEY_SCHEME_COUNT][4] = {
  {SC_CTRL, SC_ALT, SC_SPACE, 0},    /* 0 Keen-style */
  {SC_Z, SC_X, SC_C, SC_SPACE},      /* 1 Modern: C or Space fires */
  {SC_SPACE, SC_K, SC_J, 0},         /* 2 WASD: the mouse's right button also pogoes and its left also fires */
  {SC_SPACE, SC_K, SC_J, 0},         /* 3 retired (was Mouse): never reached, as keymap_scheme() turns it into 2 */
  {SC_SPACE, SC_Q, SC_E, 0},         /* 4 Left hand */
  {SC_COMMA, SC_PERIOD, SC_SLASH, 0} /* 5 Right hand */
};

/* The schemes Options offers, in order. */
static const signed char offered[] = {0, 1, 2, 4, 5};
#define OFFERED_COUNT ((int)(sizeof offered / sizeof offered[0]))

int keymap_scheme(int scheme) {
  if (scheme == KEY_SCHEME_RETIRED) return KEY_SCHEME_WASD;
  return scheme < 0 || scheme >= KEY_SCHEME_COUNT ? KEY_SCHEME_DEFAULT : scheme;
}

int keymap_step(int scheme, int dir) {
  int i, at = 0;
  scheme = keymap_scheme(scheme);
  for (i = 0; i < OFFERED_COUNT; i++)
    if (offered[i] == scheme) at = i;
  return offered[((at + (dir < 0 ? -1 : 1)) % OFFERED_COUNT + OFFERED_COUNT) % OFFERED_COUNT];
}

int keymap_is_action(int scheme, int sc) {
  const unsigned char *a = action_keys[keymap_scheme(scheme)];
  return sc != 0 && (sc == a[0] || sc == a[1] || sc == a[2] || sc == a[3]);
}

unsigned keymap_bits(const volatile unsigned char *k, int scheme) {
  const unsigned char *a = action_keys[keymap_scheme(scheme)];
  unsigned b = 0;
  if (k[SC_LEFT] || k[SC_A]) b |= IN_LEFT;
  if (k[SC_RIGHT] || k[SC_D]) b |= IN_RIGHT;
  if (k[SC_UP] || k[SC_W]) b |= IN_UP;
  if (k[SC_DOWN] || k[SC_S]) b |= IN_DOWN;
  if (a[0] && k[a[0]]) b |= IN_JUMP;
  if (a[1] && k[a[1]]) b |= IN_POGO;
  if ((a[2] && k[a[2]]) || (a[3] && k[a[3]])) b |= IN_FIRE;
  if (k[SC_ENTER]) b |= IN_CONFIRM;
  if (k[SC_ESC]) b |= IN_MENU;
  if (k[SC_F5]) b |= IN_QSAVE;
  if (k[SC_F9]) b |= IN_QLOAD;
  return b;
}
