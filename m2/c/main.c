#include <stdio.h>
#include <stdlib.h>
extern void fizz_m0(unsigned ticks, int use53);
int main(int argc, char **argv) {
  unsigned ticks = argc > 1 ? (unsigned)atoi(argv[1]) : 3000;
  int use53 = argc > 2 ? atoi(argv[2]) : 1;
  fizz_m0(ticks, use53);
  return 0;
}

/* The interface's frame rate setting lives in the full game's main loop; the hash check has none. */
int rate_mode(void) { return 0; }
void rate_set(int mode) { (void)mode; }
int rate_active(void) { return 60; }
int detail_mode(void) { return 0; }
void detail_set(int mode) { (void)mode; }
int detail_active(void) { return 0; }
void settings_save(int night) { (void)night; }
