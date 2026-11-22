/* The mouse's two buttons as game input bits, through the DOS mouse driver (INT 33h).
 *
 * (c) Copyright 2026 Liminal HQ, Scott Morris
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

#include <string.h>
#include "keymap.h"
#include "mouse.h"

static int present;
static int active;
static unsigned last_bits;

int mouse_present(void) { return present; }

unsigned mouse_bits(int status, int left_presses, int right_presses) {
  unsigned b = 0;
  if ((status & 1) || left_presses > 0) b |= IN_FIRE;
  if ((status & 2) || right_presses > 0) b |= IN_POGO;
  return b;
}

void mouse_idle(void) { active = 0; last_bits = 0; }
unsigned mouse_held(void) { return last_bits; }

#ifdef __DJGPP__
#include <dpmi.h>

/* INT 33h with AX = function and BX = argument; the registers are returned. */
static __dpmi_regs call(int function, int bx) {
  __dpmi_regs r;
  memset(&r, 0, sizeof r);
  r.x.ax = (unsigned short)function;
  r.x.bx = (unsigned short)bx;
  __dpmi_int(0x33, &r);
  return r;
}

int mouse_init(void) {
  present = call(0, 0).x.ax == 0xFFFF; /* function 0, reset: AX is FFFFh when a driver answers */
  active = 0;
  last_bits = 0;
  return present;
}

unsigned mouse_tick_bits(void) {
  int left, right, status;
  if (!present) return 0;
  /* Function 5, button press data for one button (BX 0 left, 1 right): BX is how many presses happened since the last call for that
     button, which is how a click shorter than a tick is kept. Function 3 gives the buttons held now in BX. */
  left = call(5, 0).x.bx;
  right = call(5, 1).x.bx;
  status = call(3, 0).x.bx;
  if (!active) {
    /* The mouse was not being read until now: clicks made before then are not for the game. */
    active = 1;
    left = right = 0;
  }
  last_bits = mouse_bits(status, left, right);
  return last_bits;
}
#else
/* Native checks have no driver. */
int mouse_init(void) { present = 0; return 0; }
unsigned mouse_tick_bits(void) { return 0; }
#endif
