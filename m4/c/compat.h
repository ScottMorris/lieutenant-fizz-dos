/* Interface to the start-up system check's hardware probes.
 *
 * (c) Copyright 2026 Liminal HQ, Scott Morris
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

#ifndef COMPAT_H
#define COMPAT_H
/* What the machine has, for the start-up screen. The numbers go in the same slots as the Rust side's `preview` constants. */
enum { C_VIDEO, C_MOUSE, C_JOYSTICK, C_ADLIB, C_BLASTER, C_FPU, C_MEMORY_KB, C_FPS, C_COUNT };

/* The best video adapter found, in the C_VIDEO slot: only the two VGAs have mode 13h. An MCGA, which has mode 13h too,
   is not told apart and counts as the CGA it also is. */
enum { VIDEO_UNKNOWN, VIDEO_MDA, VIDEO_CGA, VIDEO_EGA, VIDEO_VGA_MONO, VIDEO_VGA };

/* Which adapter the machine has (one of VIDEO_*). */
int compat_video(void);

/* Looks at the hardware (before the game takes its memory and before the sound handler takes the timer). `cpu` receives
   the processor's family code as uname reports it. */
void compat_probe(int v[C_COUNT], char cpu[16]);
#endif
