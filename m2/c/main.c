#include <stdio.h>
#include <stdlib.h>
extern void fizz_m0(unsigned ticks, int use53);
int main(int argc, char **argv) {
  unsigned ticks = argc > 1 ? (unsigned)atoi(argv[1]) : 3000;
  int use53 = argc > 2 ? atoi(argv[2]) : 1;
  fizz_m0(ticks, use53);
  return 0;
}
