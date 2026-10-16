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
  return b;
}
