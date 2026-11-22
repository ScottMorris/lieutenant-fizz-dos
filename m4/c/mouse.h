/* The mouse's two buttons as game input bits, through the DOS mouse driver (INT 33h).
 *
 * (c) Copyright 2026 Liminal HQ, Scott Morris
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

#ifndef MOUSE_H
#define MOUSE_H
/* Resets the mouse driver once (INT 33h function 0) and returns 1 when one is installed. The pointer is never shown and never drawn: the
   game has no pointer, only the two buttons. */
int mouse_init(void);
/* The result of mouse_init(); 0 before it has run. */
int mouse_present(void);
/* The input bits for the buttons in a scheme: the right button works the pogo (IN_POGO) in both; the left button is Fizz (IN_FIRE) in WASD and Jump (IN_JUMP) in Mouse jump. `status` is the
   driver's button word (bit 0 left, bit 1 right, set while held); a press count above zero means that button went down that many
   times since the last read, so a click shorter than a tick still gives its bit for one tick. No DOS is involved. */
unsigned mouse_bits(int scheme, int status, int left_presses, int right_presses);
/* One tick's bits, read from the driver (button status and each button's press count): the held buttons plus any click since the last
   tick. It is only for the WASD and Mouse jump schemes with a mouse found at start-up; a first call after it was not in use drops the clicks made meanwhile. Returns 0
   without a mouse. */
unsigned mouse_tick_bits(int scheme);
/* The bits of the last mouse_tick_bits(), for the ticks of a catch-up that follow the first one. */
unsigned mouse_held(void);
/* Tells the module the mouse is not being read (any scheme but WASD and Mouse jump), so nothing is read from the driver and the next use starts clean. */
void mouse_idle(void);
#endif
