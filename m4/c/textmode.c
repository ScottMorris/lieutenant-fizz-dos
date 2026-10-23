/* The 80x25 text screen: sets the mode, hides the cursor, turns blinking off and writes cells straight to video memory.
 *
 * (c) Copyright 2026 Liminal HQ, Scott Morris
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

#include <dpmi.h>
#include <go32.h>
#include <string.h>
#include <sys/movedata.h>
#include "textmode.h"

static int mode = 3;

static void set_mode(int m) {
  __dpmi_regs r;
  memset(&r, 0, sizeof r);
  r.x.ax = (unsigned short)m;
  __dpmi_int(0x10, &r);
}

/* A colour adapter (CGA, EGA, VGA) has its text at 0xB8000 in mode 3. A monochrome one (MDA, Hercules, or an EGA or VGA on
   a monochrome display) starts in mode 7, which has no mode 3, with its text at 0xB0000; its attributes come out as normal,
   bright, underlined or inverse text rather than colours. */
void text_open(void) {
  __dpmi_regs r;
  memset(&r, 0, sizeof r);
  r.h.ah = 0x0F;
  __dpmi_int(0x10, &r);
  mode = (r.h.al & 0x7F) == 7 ? 7 : 3;
  set_mode(mode);
  /* The cursor off: start line 0x20 is past the bottom of the cell. */
  memset(&r, 0, sizeof r);
  r.h.ah = 0x01;
  r.x.cx = 0x2000;
  __dpmi_int(0x10, &r);
  /* Bit 7 of an attribute as a bright background rather than blinking (INT 10h AX=1003h BL=0, EGA and VGA). The cells never
     set it, so on an older adapter, which ignores the call, nothing blinks either. */
  memset(&r, 0, sizeof r);
  r.x.ax = 0x1003;
  r.h.bl = 0;
  __dpmi_int(0x10, &r);
}

int text_mode(void) { return mode; }

unsigned long text_base(void) { return mode == 7 ? 0xB0000UL : 0xB8000UL; }

void text_show(const unsigned short cells[TEXT_CELLS]) { dosmemput(cells, TEXT_CELLS * 2, text_base()); }

void text_read(unsigned short cells[TEXT_CELLS]) { dosmemget(text_base(), TEXT_CELLS * 2, cells); }

void text_close(void) { set_mode(mode); }
