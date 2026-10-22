/* Maps keyboard scancodes to the game's input bits.
 *
 * (c) Copyright 2026 Liminal HQ, Scott Morris
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

#include "keymap.h"

unsigned keymap_bits(const volatile unsigned char *k) {
  unsigned b = 0;
  if (k[SC_LEFT] || k[SC_A]) b |= IN_LEFT;
  if (k[SC_RIGHT] || k[SC_D]) b |= IN_RIGHT;
  if (k[SC_UP] || k[SC_W]) b |= IN_UP;
  if (k[SC_DOWN] || k[SC_S]) b |= IN_DOWN;
  if (k[SC_CTRL] || k[SC_Z]) b |= IN_JUMP;
  if (k[SC_ALT] || k[SC_X]) b |= IN_POGO;
  if (k[SC_SPACE] || k[SC_C]) b |= IN_FIRE;
  if (k[SC_ENTER]) b |= IN_CONFIRM;
  if (k[SC_ESC]) b |= IN_MENU;
  if (k[SC_F5]) b |= IN_QSAVE;
  if (k[SC_F9]) b |= IN_QLOAD;
  return b;
}
