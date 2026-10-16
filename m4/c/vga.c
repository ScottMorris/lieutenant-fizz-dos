/* VGA mode 13h: sets the mode, programs the palette and copies a frame to video memory.
 *
 * (c) Copyright 2026 Liminal HQ, Scott Morris
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

#include <dpmi.h>
#include <pc.h>
#include <string.h>
#include <sys/nearptr.h>
#include "vga.h"

static void set_mode(int m) {
  __dpmi_regs r;
  r.x.ax = (unsigned short)m;
  __dpmi_int(0x10, &r);
}

int vga_open(void) {
  if (!__djgpp_nearptr_enable()) return 1;
  set_mode(0x13);
  return 0;
}

void vga_close(void) {
  set_mode(0x03);
  __djgpp_nearptr_disable();
}

void vga_palette(int i, int r6, int g6, int b6) {
  outportb(0x3C8, (unsigned char)i);
  outportb(0x3C9, (unsigned char)r6);
  outportb(0x3C9, (unsigned char)g6);
  outportb(0x3C9, (unsigned char)b6);
}

/* Wait for the start of the vertical retrace: bit 3 of the input status register. */
void vga_vsync(void) {
  while (inportb(0x3DA) & 8) {}
  while (!(inportb(0x3DA) & 8)) {}
}

void vga_present(const unsigned char *frame) {
  /* The near-pointer base can change whenever the heap grows, so it is read every frame. */
  memcpy((unsigned char *)(__djgpp_conventional_base + 0xA0000), frame, 64000);
}
