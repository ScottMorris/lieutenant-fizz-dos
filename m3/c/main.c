/* Episode 1 on a VGA mode 13h screen.
   usage: FIZZ [level] [ticks] [shot] [nosync]
   With no ticks (or 0) it is interactive: arrows or WASD move, Ctrl or Z jump, Alt or X pogo, Space or C fire,
   PgUp and PgDn change level, R restarts it, P pauses, Esc quits. With ticks it runs a scripted walk for that many
   ticks and, with "shot", writes FRAME.RAW (64000 bytes) and PAL.RAW (18 x RGB, 6-bit) at the end. */
#include <conio.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include "kbd.h"
#include "keymap.h"
#include "vga.h"

extern void fizz_start(unsigned level, int use53);
extern unsigned fizz_scripted(unsigned tick);
extern void fizz_tick(unsigned held);
extern void fizz_goto(unsigned level);
extern unsigned fizz_level(void);
extern unsigned fizz_draw(unsigned char *frame, float alpha);

/* The 16 EGA colours as 6-bit VGA channels, in the sprite DSL's order (k B G C R M N L D b g c r m y W). */
static const unsigned char ega[16][3] = {
  {0, 0, 0},   {0, 0, 42},  {0, 42, 0},  {0, 42, 42}, {42, 0, 0},  {42, 0, 42}, {42, 21, 0}, {42, 42, 42},
  {21, 21, 21}, {21, 21, 63}, {21, 63, 21}, {21, 63, 63}, {63, 21, 21}, {63, 21, 63}, {63, 63, 21}, {63, 63, 63}};

static unsigned char frame[64000];

static void show(unsigned *clear, int sync, float alpha) {
  *clear = fizz_draw(frame, alpha);
  vga_palette(17, (*clear >> 16) & 63, (*clear >> 8) & 63, *clear & 63);
  if (sync) vga_vsync();
  vga_present(frame);
}

int main(int argc, char **argv) {
  unsigned level = argc > 1 ? (unsigned)atoi(argv[1]) : 0;
  unsigned ticks = argc > 2 ? (unsigned)atoi(argv[2]) : 0;
  int shot = 0, nosync = 0, i, paused = 0;
  unsigned t, clear = 0;
  unsigned char pal[18 * 3];
  for (i = 3; i < argc; i++) {
    if (!strcmp(argv[i], "shot")) shot = 1;
    if (!strcmp(argv[i], "nosync")) nosync = 1;
  }
  fizz_start(level, 1);
  if (ticks == 0 && kbd_install()) { printf("could not hook the keyboard\n"); return 1; }
  if (vga_open()) { printf("no near pointers\n"); kbd_remove(); return 1; }
  vga_palette(0, 0, 0, 0);
  for (i = 0; i < 16; i++) vga_palette(i + 1, ega[i][0], ega[i][1], ega[i][2]);

  if (ticks) {
    /* A scripted run: one tick a frame, so a screenshot is the same every time. */
    for (t = 0; t < ticks; t++) {
      fizz_tick(fizz_scripted(t));
      show(&clear, !nosync, 1.0f);
    }
  } else {
    /* Interactive: the sim runs at 60 Hz from the PIT clock, the screen follows the retrace and draws between ticks. */
    const uclock_t step = UCLOCKS_PER_SEC / 60;
    uclock_t last = uclock(), acc = 0;
    t = 0;
    for (;;) {
      uclock_t now = uclock();
      acc += now - last;
      last = now;
      if (acc > 6 * step) acc = 6 * step; /* after a stall, do not rush to catch up */
      if (kbd_pressed(SC_ESC)) break;
      if (kbd_pressed(SC_P)) paused = !paused;
      if (kbd_pressed(SC_PGUP)) fizz_goto(fizz_level() + 15);
      if (kbd_pressed(SC_PGDN)) fizz_goto(fizz_level() + 1);
      if (kbd_pressed(SC_R)) fizz_goto(fizz_level());
      while (acc >= step) {
        if (!paused) { fizz_tick(kbd_held()); t++; }
        acc -= step;
      }
      show(&clear, 1, paused ? 1.0f : (float)acc / (float)step);
    }
  }
  vga_close();
  kbd_remove();
  if (shot) {
    FILE *f = fopen("FRAME.RAW", "wb");
    if (f) { fwrite(frame, 1, 64000, f); fclose(f); }
    for (i = 0; i < 16; i++) { pal[i * 3 + 3] = ega[i][0]; pal[i * 3 + 4] = ega[i][1]; pal[i * 3 + 5] = ega[i][2]; }
    pal[0] = pal[1] = pal[2] = 0;
    pal[51] = (clear >> 16) & 63; pal[52] = (clear >> 8) & 63; pal[53] = clear & 63;
    f = fopen("PAL.RAW", "wb");
    if (f) { fwrite(pal, 1, sizeof pal, f); fclose(f); }
  }
  printf("%u ticks, ended on level %u\n", t, fizz_level());
  return 0;
}
