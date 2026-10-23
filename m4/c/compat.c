/* The start-up system check's probes: CPU, FPU, video, input and sound hardware.
 *
 * (c) Copyright 2026 Liminal HQ, Scott Morris
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

#include <dos.h>
#include <dpmi.h>
#include <go32.h>
#include <pc.h>
#include <string.h>
#include <sys/utsname.h>
#include "compat.h"
#include "sound.h"

static int fpu_present(void) {
  unsigned short cw = 0x5A5A;
  __asm__ volatile("fninit; fnstcw %0" : "=m"(cw));
  return (cw & 0x103F) == 0x003F;
}

/* The display combination call (INT 10h AX=1A00h) answers 1Ah on a VGA, with BL 7 for a monochrome display and 8 for
   colour. Without it, the EGA information call (INT 10h AH=12h BL=10h) changes BL on an EGA; failing both, the BIOS
   equipment word (INT 11h) bits 4-5 say whether the start-up display was colour (01 or 10, a CGA) or monochrome (11, an
   MDA or Hercules). */
int compat_video(void) {
  __dpmi_regs r;
  int display;
  memset(&r, 0, sizeof r);
  r.x.ax = 0x1A00;
  __dpmi_int(0x10, &r);
  if (r.h.al == 0x1A && r.h.bl == 8) return VIDEO_VGA;
  if (r.h.al == 0x1A && r.h.bl == 7) return VIDEO_VGA_MONO;
  memset(&r, 0, sizeof r);
  r.h.ah = 0x12;
  r.h.bl = 0x10;
  __dpmi_int(0x10, &r);
  if (r.h.bl != 0x10) return VIDEO_EGA;
  memset(&r, 0, sizeof r);
  __dpmi_int(0x11, &r);
  display = (r.x.ax >> 4) & 3;
  if (display == 3) return VIDEO_MDA;
  if (display != 0) return VIDEO_CGA;
  return VIDEO_UNKNOWN;
}

void compat_probe(int v[C_COUNT], char cpu[16]) {
  __dpmi_regs r;
  struct utsname u;
  unsigned long kb;
  __dpmi_free_mem_info info;
  memset(v, 0, C_COUNT * sizeof(int));
  v[C_VIDEO] = compat_video();
  memset(&r, 0, sizeof r);
  r.x.ax = 0;
  __dpmi_int(0x33, &r);
  v[C_MOUSE] = r.x.ax == 0xFFFF;
  v[C_JOYSTICK] = inportb(0x201) != 0xFF;
  v[C_ADLIB] = snd_detect_opl();
  v[C_BLASTER] = snd_detect_blaster();
  v[C_FPU] = fpu_present();
  kb = _go32_dpmi_remaining_physical_memory() / 1024UL;
  /* When the program itself does not fit (CWSDPMI has paged part of it out), the count wraps round to a huge number: more
     free memory than the machine has means none. */
  if (__dpmi_get_free_memory_information(&info) == 0 && info.total_number_of_physical_pages != 0xFFFFFFFFUL &&
      kb > info.total_number_of_physical_pages * 4UL)
    kb = 0;
  v[C_MEMORY_KB] = kb > 0x7FFFFFUL ? 0x7FFFFF : (int)kb;
  if (uname(&u) == 0) strncpy(cpu, u.machine, 15); else strcpy(cpu, "x86");
  cpu[15] = 0;
}
