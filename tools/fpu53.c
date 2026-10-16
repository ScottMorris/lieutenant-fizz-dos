/* LD_PRELOAD shim that sets the x87 precision control to 53 bits before main().
 *
 * (c) Copyright 2026 Liminal HQ, Scott Morris
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

/* LD_PRELOAD shim for 32-bit x86 Linux: sets the x87 precision control to a 53-bit mantissa before main(). */
static void __attribute__((constructor)) fpu53(void) {
  unsigned short cw = 0x027F;
  __asm__ volatile("fldcw %0" : : "m"(cw));
}
