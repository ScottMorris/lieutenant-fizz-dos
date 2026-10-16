/* Milestone 3: Episode 1 on a VGA mode 13h screen. usage: FIZZ [level] [ticks] [shot] [nosync]
   ticks 0 runs until Esc. "shot" writes FRAME.RAW (64000 bytes) and PAL.RAW (18 x RGB, 6-bit) at the end. */
#include <conio.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "vga.h"

extern void fizz_start(unsigned level, int use53);
extern void fizz_tick(unsigned tick);
extern unsigned fizz_draw(unsigned char *frame);

/* The 16 EGA colours as 6-bit VGA channels, in the sprite DSL's order (k B G C R M N L D b g c r m y W). */
static const unsigned char ega[16][3] = {
  {0, 0, 0},   {0, 0, 42},  {0, 42, 0},  {0, 42, 42}, {42, 0, 0},  {42, 0, 42}, {42, 21, 0}, {42, 42, 42},
  {21, 21, 21}, {21, 21, 63}, {21, 63, 21}, {21, 63, 63}, {63, 21, 21}, {63, 21, 63}, {63, 63, 21}, {63, 63, 63}};

int main(int argc, char **argv) {
  unsigned level = argc > 1 ? (unsigned)atoi(argv[1]) : 0;
  unsigned ticks = argc > 2 ? (unsigned)atoi(argv[2]) : 0;
  int shot = 0, nosync = 0, i;
  unsigned t, clear = 0;
  unsigned char *frame = malloc(64000);
  unsigned char pal[18 * 3];
  for (i = 3; i < argc; i++) {
    if (!strcmp(argv[i], "shot")) shot = 1;
    if (!strcmp(argv[i], "nosync")) nosync = 1;
  }
  if (!frame) return 2;
  fizz_start(level, 1);
  if (vga_open()) { printf("no near pointers\n"); return 1; }
  vga_palette(0, 0, 0, 0);
  for (i = 0; i < 16; i++) vga_palette(i + 1, ega[i][0], ega[i][1], ega[i][2]);
  for (t = 0; ticks == 0 || t < ticks; t++) {
    fizz_tick(t);
    clear = fizz_draw(frame);
    vga_palette(17, (clear >> 16) & 63, (clear >> 8) & 63, clear & 63);
    if (!nosync) vga_vsync();
    vga_present(frame);
    if (kbhit() && getch() == 27) break;
  }
  vga_close();
  if (shot) {
    FILE *f = fopen("FRAME.RAW", "wb");
    if (f) { fwrite(frame, 1, 64000, f); fclose(f); }
    for (i = 0; i < 16; i++) { pal[i * 3 + 3] = ega[i][0]; pal[i * 3 + 4] = ega[i][1]; pal[i * 3 + 5] = ega[i][2]; }
    pal[0] = pal[1] = pal[2] = 0;
    pal[51] = (clear >> 16) & 63; pal[52] = (clear >> 8) & 63; pal[53] = clear & 63;
    f = fopen("PAL.RAW", "wb");
    if (f) { fwrite(pal, 1, sizeof pal, f); fclose(f); }
  }
  printf("%u frames on level %u\n", t, level);
  return 0;
}
