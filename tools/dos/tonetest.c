/* A test tone for checking a DOS emulator's sound: an A4 (440 Hz) note on the AdLib's OPL2, then the same pitch on the PC speaker.
 *
 * (c) Copyright 2026 Liminal HQ, Scott Morris
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 *
 * Run it at the DOS prompt (TONETEST). Each tone lasts three seconds, with half a second of silence between them.
 *
 * The OPL note is a sine from one operator pair: F-number 580 in block 4, which is 580 * 49716 / 2^16 = 440 Hz. The speaker note is
 * channel 2 of the timer as a square wave with the divisor 1193182 / 440. In an emulator that is right, both come out at the same
 * pitch, an A above middle C; measured through js-dos in a browser, the AdLib note is 440.0 Hz and the speaker note 393.5 Hz.
 */
#include <dos.h>
#include <pc.h>
#include <stdio.h>

/* Writes one OPL2 register, waiting out the chip's settling time by reading the status port. */
static void opl(int reg, int val) {
  int i;
  outportb(0x388, reg);
  for (i = 0; i < 6; i++) inportb(0x388);
  outportb(0x389, val);
  for (i = 0; i < 35; i++) inportb(0x388);
}

int main(void) {
  const int fnum = 580, block = 4;
  opl(0x01, 0x20); /* waveform select on */
  /* Modulator: attenuated to silence. Carrier: a sustained sine at full level. Additive connection, so the carrier is heard. */
  opl(0x20, 0x21); opl(0x40, 0x3f); opl(0x60, 0xf0); opl(0x80, 0x0f); opl(0xe0, 0x00);
  opl(0x23, 0x21); opl(0x43, 0x00); opl(0x63, 0xf0); opl(0x83, 0x0f); opl(0xe3, 0x00);
  opl(0xc0, 0x01);
  opl(0xa0, fnum & 255);
  opl(0xb0, 0x20 | (block << 2) | (fnum >> 8)); /* key on */
  printf("AdLib (OPL2), A4 = 440 Hz, three seconds\n");
  delay(3000);
  opl(0xb0, 0x00); /* key off */
  delay(500);

  outportb(0x43, 0xb6); /* timer channel 2, square wave */
  outportb(0x42, (1193182 / 440) & 255);
  outportb(0x42, (1193182 / 440) >> 8);
  outportb(0x61, inportb(0x61) | 3);
  printf("PC speaker, A4 = 440 Hz, three seconds\n");
  delay(3000);
  outportb(0x61, inportb(0x61) & ~3);
  printf("done\n");
  return 0;
}
