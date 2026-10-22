/* Interface to the save and options files.
 *
 * (c) Copyright 2026 Liminal HQ, Scott Morris
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

#ifndef SAVE_H
#define SAVE_H
/* Save files in the current directory: slot 0 is AUTO.SAV, slots 1 to 4 are SLOT1.SAV to SLOT4.SAV, slot 5 is QUICK.SAV (F5 and F9; save.rs has the same numbers), and slot -1 is FIZZ.CFG (the options). */
int save_write(int slot, const unsigned char *data, int length); /* 1 on success */
int save_read(int slot, unsigned char *data, int length);         /* bytes read, 0 when there is none */
unsigned long save_stamp(void);                                    /* seconds since 1970 */
#endif
