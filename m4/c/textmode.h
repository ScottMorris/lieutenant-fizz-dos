/* Interface to the 80x25 text screen that the start-up check can be shown on.
 *
 * (c) Copyright 2026 Liminal HQ, Scott Morris
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

#ifndef TEXTMODE_H
#define TEXTMODE_H
#define TEXT_CELLS 2000 /* 80 x 25: a character in the low byte, its attribute in the high byte */

/* Sets the 80x25 text mode (mode 3, or mode 7 on a monochrome adapter), hides the cursor and turns blinking off. */
void text_open(void);
/* Writes a whole screen of cells to video memory. */
void text_show(const unsigned short cells[TEXT_CELLS]);
/* Reads the screen back from video memory, for a check. */
void text_read(unsigned short cells[TEXT_CELLS]);
/* The BIOS video mode in use and where its cells are (0xB8000 or 0xB0000). */
int text_mode(void);
unsigned long text_base(void);
/* Sets the text mode again, which clears the screen and brings the cursor and blinking back. */
void text_close(void);
#endif
