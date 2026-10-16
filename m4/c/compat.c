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

void compat_probe(int v[C_COUNT], char cpu[16]) {
  __dpmi_regs r;
  struct utsname u;
  memset(v, 0, C_COUNT * sizeof(int));
  /* Display combination: VGA is 7 (mono) or 8 (colour). */
  memset(&r, 0, sizeof r);
  r.x.ax = 0x1A00;
  __dpmi_int(0x10, &r);
  v[C_VGA] = r.h.al == 0x1A && (r.h.bl == 7 || r.h.bl == 8);
  /* EGA information: BL changes from 0x10 when an EGA or VGA is there. */
  memset(&r, 0, sizeof r);
  r.h.ah = 0x12;
  r.h.bl = 0x10;
  __dpmi_int(0x10, &r);
  v[C_EGA] = r.h.bl != 0x10;
  memset(&r, 0, sizeof r);
  r.x.ax = 0;
  __dpmi_int(0x33, &r);
  v[C_MOUSE] = r.x.ax == 0xFFFF;
  v[C_JOYSTICK] = inportb(0x201) != 0xFF;
  v[C_ADLIB] = snd_detect_opl();
  v[C_BLASTER] = snd_detect_blaster();
  v[C_FPU] = fpu_present();
  v[C_MEMORY_KB] = (int)(_go32_dpmi_remaining_physical_memory() / 1024UL);
  if (uname(&u) == 0) strncpy(cpu, u.machine, 15); else strcpy(cpu, "x86");
  cpu[15] = 0;
}
