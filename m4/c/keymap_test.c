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
int main(void) {
  volatile unsigned char k[128];
  memset((void *)k, 0, sizeof k);
  expect("nothing held", keymap_bits(k), 0);
  k[SC_RIGHT] = 1; expect("right arrow", keymap_bits(k), IN_RIGHT);
  k[SC_CTRL] = 1; expect("right and Ctrl", keymap_bits(k), IN_RIGHT | IN_JUMP);
  k[SC_RIGHT] = 0; k[SC_CTRL] = 0; k[SC_D] = 1; k[SC_Z] = 1; k[SC_C] = 1;
  expect("WASD and Z and C", keymap_bits(k), IN_RIGHT | IN_JUMP | IN_FIRE);
  memset((void *)k, 0, sizeof k); k[SC_ALT] = 1; k[SC_UP] = 1; k[SC_LEFT] = 1;
  expect("Alt up left", keymap_bits(k), IN_POGO | IN_UP | IN_LEFT);
  memset((void *)k, 0, sizeof k); k[SC_PGUP] = 1;
  expect("keys that are not game input", keymap_bits(k), 0);
  memset((void *)k, 0, sizeof k); k[SC_ESC] = 1;
  expect("Escape is the menu key", keymap_bits(k), IN_MENU);
  memset((void *)k, 0, sizeof k); k[SC_ENTER] = 1;
  expect("Enter confirms", keymap_bits(k), IN_CONFIRM);
  if (!failed) printf("keymap ok\n");
  return failed;
}
