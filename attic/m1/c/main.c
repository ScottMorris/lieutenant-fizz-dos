#include <stdio.h>
extern double rust_hello(void);
int main(void) {
  printf("start\n"); fflush(stdout);
  double r = rust_hello();
  printf("rust_hello returned %f\n", r);
  return 0;
}
