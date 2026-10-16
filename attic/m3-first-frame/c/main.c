/* Episode 1 on a VGA mode 13h screen.
   usage: FIZZ [level] [ticks] [shot] [nosync] [night]
   With no ticks (or 0) it is interactive: arrows or WASD move, Ctrl or Z jump, Alt or X pogo, Space or C fire,
   PgUp and PgDn change level, R restarts it, N toggles night, P pauses, Esc quits. With ticks it runs a scripted walk for that many
   ticks and, with "shot", writes FRAME.RAW (64000 bytes) and PAL.RAW (256 x RGB, 6-bit) at the end. */
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
extern void fizz_night(int on);
extern unsigned fizz_level(void);
extern unsigned fizz_build(float alpha);
extern unsigned fizz_paint(unsigned char *frame, unsigned count);

/* The 16 EGA colours as 6-bit VGA channels, in the sprite DSL's order (k B G C R M N L D b g c r m y W). */
static const unsigned char ega[16][3] = {
  {0, 0, 0},   {0, 0, 42},  {0, 42, 0},  {0, 42, 42}, {42, 0, 0},  {42, 0, 42}, {42, 21, 0}, {42, 42, 42},
  {21, 21, 21}, {21, 21, 63}, {21, 63, 21}, {21, 63, 63}, {63, 21, 21}, {63, 21, 63}, {63, 63, 21}, {63, 63, 63}};

static unsigned char frame[64000];

/* The palette: byte = level * 16 + colour. Levels 1..14 dim the EGA colour to (level - 1) / 13 of itself; level 0 is
   the colour at full brightness (what a byte means before the lighting pass); slot 255 is the level's clear colour. */
static void build_palette(unsigned char pal[768]) {
  int lvl, c;
  memset(pal, 0, 768);
  for (lvl = 0; lvl < 15; lvl++)
    for (c = 0; c < 16; c++) {
      int k;
      for (k = 0; k < 3; k++)
        pal[(lvl * 16 + c) * 3 + k] = lvl == 0 ? ega[c][k] : (unsigned char)((ega[c][k] * (lvl - 1) + 6) / 13);
    }
}
static uclock_t t_tick, t_build, t_draw, t_present;
static unsigned n_sprites;
static unsigned n_frames;

static void show(unsigned *clear, int sync, float alpha) {
  uclock_t a = uclock(), m, b, c;
  unsigned count = fizz_build(alpha);
  m = uclock();
  *clear = fizz_paint(frame, count);
  b = uclock();
  n_sprites += count;
  t_build += m - a;
  vga_palette(255, (*clear >> 16) & 63, (*clear >> 8) & 63, *clear & 63);
  if (sync) vga_vsync();
  vga_present(frame);
  c = uclock();
  t_draw += b - m;
  t_present += c - b;
  n_frames++;
}

int main(int argc, char **argv) {
  unsigned level = argc > 1 ? (unsigned)atoi(argv[1]) : 0;
  unsigned ticks = argc > 2 ? (unsigned)atoi(argv[2]) : 0;
  int shot = 0, nosync = 0, i, paused = 0, night = 0;
  unsigned t, clear = 0;
  unsigned char pal[768];
  for (i = 3; i < argc; i++) {
    if (!strcmp(argv[i], "shot")) shot = 1;
    if (!strcmp(argv[i], "nosync")) nosync = 1;
    if (!strcmp(argv[i], "night")) night = 1;
  }
  fizz_start(level, 1);
  fizz_night(night);
  if (ticks == 0 && kbd_install()) { printf("could not hook the keyboard\n"); return 1; }
  if (vga_open()) { printf("no near pointers\n"); kbd_remove(); return 1; }
  build_palette(pal);
  for (i = 0; i < 255; i++) vga_palette(i, pal[i * 3], pal[i * 3 + 1], pal[i * 3 + 2]);

  if (ticks) {
    /* A scripted run: one tick a frame, so a screenshot is the same every time. */
    for (t = 0; t < ticks; t++) {
      { uclock_t a = uclock(); fizz_tick(fizz_scripted(t)); t_tick += uclock() - a; }
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
      if (kbd_pressed(SC_N)) { night = !night; fizz_night(night); }
      if (acc >= step) {
        /* The first tick of a frame also gets any quick taps; the rest of a catch-up see what is held. */
        unsigned bits = kbd_tick_bits();
        while (acc >= step) {
          if (!paused) { uclock_t a = uclock(); fizz_tick(bits); t_tick += uclock() - a; t++; }
          bits = kbd_held();
          acc -= step;
        }
      }
      show(&clear, 1, paused ? 1.0f : (float)acc / (float)step);
    }
  }
  vga_close();
  kbd_remove();
  if (shot) {
    FILE *f = fopen("FRAME.RAW", "wb");
    if (f) { fwrite(frame, 1, 64000, f); fclose(f); }
    pal[765] = (clear >> 16) & 63; pal[766] = (clear >> 8) & 63; pal[767] = clear & 63;
    f = fopen("PAL.RAW", "wb");
    if (f) { fwrite(pal, 1, sizeof pal, f); fclose(f); }
  }
  printf("%u ticks, ended on level %u\n", t, fizz_level());
  if (n_frames)
    printf("per frame: build %lu us, paint %lu us (%u sprites), present %lu us; per tick: %lu us\n",
           (unsigned long)(t_build * 1000000ULL / UCLOCKS_PER_SEC / n_frames),
           (unsigned long)(t_draw * 1000000ULL / UCLOCKS_PER_SEC / n_frames), n_sprites / n_frames,
           (unsigned long)(t_present * 1000000ULL / UCLOCKS_PER_SEC / n_frames),
           t ? (unsigned long)(t_tick * 1000000ULL / UCLOCKS_PER_SEC / t) : 0UL);
  return 0;
}
