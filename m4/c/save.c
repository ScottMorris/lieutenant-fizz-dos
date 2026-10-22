/* Files in the current directory: game saves and the options file.
 *
 * (c) Copyright 2026 Liminal HQ, Scott Morris
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

#include <stdio.h>
#include <time.h>
#include "save.h"

static void name_of(int slot, char name[16]) {
  if (slot < 0) sprintf(name, "FIZZ.CFG");
  else if (slot == 0) sprintf(name, "AUTO.SAV");
  else if (slot == 5) sprintf(name, "QUICK.SAV");
  else sprintf(name, "SLOT%d.SAV", slot);
}

int save_write(int slot, const unsigned char *data, int length) {
  char name[16];
  FILE *f;
  int ok;
  name_of(slot, name);
  f = fopen(name, "wb");
  if (!f) return 0;
  ok = fwrite(data, 1, (size_t)length, f) == (size_t)length;
  fclose(f);
  return ok;
}

int save_read(int slot, unsigned char *data, int length) {
  char name[16];
  FILE *f;
  int got;
  name_of(slot, name);
  f = fopen(name, "rb");
  if (!f) return 0;
  got = (int)fread(data, 1, (size_t)length, f);
  fclose(f);
  return got;
}

unsigned long save_stamp(void) { return (unsigned long)time(NULL); }
