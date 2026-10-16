/* Interface to the VGA mode 13h screen.
 *
 * (c) Copyright 2026 Liminal HQ, Scott Morris
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

#ifndef VGA_H
#define VGA_H
/* VGA mode 13h: 320x200, 256 colours, one byte a pixel at 0xA0000. */
int vga_open(void);                 /* enable near pointers, set mode 13h; 0 on success */
void vga_close(void);               /* back to 80x25 text mode */
void vga_palette(int i, int r6, int g6, int b6);
void vga_vsync(void);
void vga_present(const unsigned char *frame); /* copies 64000 bytes to the screen */
#endif
