/* Episode 1 on a VGA mode 13h screen, with sound.
   usage: FIZZ [level] [ticks] [shot] [nosync] [night] [nosound] [adlib] [speaker] [soundtest]
   With no ticks (or 0) it is interactive: arrows or WASD move, Ctrl or Z jump, Alt or X pogo, Space or C fire,
   PgUp and PgDn change level, R restarts it, N toggles night, M music, E effects, P pauses, Esc quits.
   With ticks it runs a scripted walk for that many ticks and, with "shot", writes FRAME.RAW (64000 bytes) and PAL.RAW
   (256 x RGB, 6-bit) at the end. "soundtest" plays a song for a few seconds headless and reports what the driver did. */
#include <conio.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include "kbd.h"
#include "keymap.h"
#include "sound.h"
#include "vga.h"

extern void fizz_start(unsigned level, int use53);
extern unsigned fizz_scripted(unsigned tick);
extern void fizz_tick(unsigned held);
extern void fizz_goto(unsigned level);
extern unsigned fizz_level(void);
extern unsigned fizz_boss_hp(void);
extern void fizz_night(int on);
extern unsigned fizz_build(float alpha);
extern unsigned fizz_paint(unsigned char *frame, unsigned count);
extern const unsigned char *fizz_sound_data(void);
extern unsigned long fizz_sound_len(void);

/* The 16 EGA colours as 6-bit VGA channels, in the sprite DSL's order (k B G C R M N L D b g c r m y W). */
static const unsigned char ega[16][3] = {
  {0, 0, 0},   {0, 0, 42},  {0, 42, 0},  {0, 42, 42}, {42, 0, 0},  {42, 0, 42}, {42, 21, 0}, {42, 42, 42},
  {21, 21, 21}, {21, 21, 63}, {21, 63, 21}, {21, 63, 63}, {63, 21, 21}, {63, 21, 63}, {63, 63, 21}, {63, 63, 63}};

static unsigned char frame[64000];
static uclock_t t_tick, t_build, t_draw, t_present;
static unsigned n_sprites, n_frames;

/* uclock() reads the BIOS timer, which the sound handler has taken over, so it is only used when that is not running. */
static uclock_t stamp(void) { return snd_mode() != SND_OFF ? 0 : uclock(); }

/* The palette: byte = level * 16 + colour. Levels 1..14 dim the EGA colour to (level - 1) / 13 of itself; level 0 is
   the colour at full brightness (what a byte means before the lighting pass); slot 255 is the level's clear colour. */
static void build_palette(unsigned char pal[768]) {
  int lvl, c, k;
  memset(pal, 0, 768);
  for (lvl = 0; lvl < 15; lvl++)
    for (c = 0; c < 16; c++)
      for (k = 0; k < 3; k++)
        pal[(lvl * 16 + c) * 3 + k] = lvl == 0 ? ega[c][k] : (unsigned char)((ega[c][k] * (lvl - 1) + 6) / 13);
}

static void show(unsigned *clear, int sync, float alpha) {
  uclock_t a = stamp(), m, b, c;
  unsigned count = fizz_build(alpha);
  m = stamp();
  *clear = fizz_paint(frame, count);
  b = stamp();
  n_sprites += count;
  t_build += m - a;
  vga_palette(255, (*clear >> 16) & 63, (*clear >> 8) & 63, *clear & 63);
  if (sync) vga_vsync();
  vga_present(frame);
  c = stamp();
  t_draw += b - m;
  t_present += c - b;
  n_frames++;
}

/* The game's clock in 1/560 s: the sound handler's count when it is running, else the BIOS clock. */
static unsigned long clock560(void) {
  if (snd_mode() != SND_OFF) return snd_ticks();
  return (unsigned long)((uclock() * 560ULL) / UCLOCKS_PER_SEC);
}

static void choose_music(void) {
  static int last_level = -2, last_boss = -2;
  int level = (int)fizz_level(), boss = fizz_boss_hp() > 0;
  if (level != last_level || boss != last_boss) {
    last_level = level;
    last_boss = boss;
    snd_music_for(level, boss);
  }
}

int main(int argc, char **argv) {
  unsigned level = argc > 1 ? (unsigned)atoi(argv[1]) : 0;
  unsigned ticks = argc > 2 ? (unsigned)atoi(argv[2]) : 0;
  int shot = 0, nosync = 0, i, paused = 0, night = 0, sound_mode = SND_AUTO, soundtest = 0, with_sound = 0, quiet = 0, bench = 0;
  unsigned t, clear = 0;
  unsigned char pal[768];
  for (i = 3; i < argc; i++) {
    if (!strcmp(argv[i], "shot")) shot = 1;
    if (!strcmp(argv[i], "nosync")) nosync = 1;
    if (!strcmp(argv[i], "night")) night = 1;
    if (!strcmp(argv[i], "nosound")) sound_mode = SND_OFF;
    if (!strcmp(argv[i], "adlib")) sound_mode = SND_ADLIB;
    if (!strcmp(argv[i], "speaker")) sound_mode = SND_SPEAKER;
    if (!strcmp(argv[i], "soundtest")) soundtest = 1;
    if (!strcmp(argv[i], "sound")) with_sound = 1;
    if (!strcmp(argv[i], "quiet")) quiet = 1;
    if (!strcmp(argv[i], "bench")) bench = 1;
  }
  fizz_start(level, 1);
  fizz_night(night);

  if (soundtest) {
    /* No screen: start the handler, play the first song for three seconds and report what it did. */
    unsigned long start, waited = 0;
    int m = snd_init(fizz_sound_data(), fizz_sound_len(), sound_mode);
    printf("sound mode %d (0 auto, 1 adlib, 2 speaker, 3 off)\n", m);
    if (m == SND_OFF) return 1;
    snd_music(0);
    snd_fx(0);
    start = snd_ticks();
    while (snd_ticks() - start < 560UL * 3 && waited < 2000000000UL) waited++;
    printf("ticks %lu, register writes %lu\n", snd_ticks() - start, snd_register_writes());
    snd_shutdown();
    return 0;
  }

  if (ticks == 0 || with_sound) {
    if (sound_mode != SND_OFF) snd_init(fizz_sound_data(), fizz_sound_len(), sound_mode);
    if ((ticks || bench) && !quiet) snd_music(0);
  }
  if (ticks == 0 && !bench) {
    if (kbd_install()) { printf("could not hook the keyboard\n"); snd_shutdown(); return 1; }
  }
  if (vga_open()) { printf("no near pointers\n"); kbd_remove(); snd_shutdown(); return 1; }
  build_palette(pal);
  for (i = 0; i < 255; i++) vga_palette(i, pal[i * 3], pal[i * 3 + 1], pal[i * 3 + 2]);

  if (ticks) {
    /* A scripted run: one tick a frame, so a screenshot is the same every time. */
    for (t = 0; t < ticks; t++) {
      { uclock_t a = stamp(); fizz_tick(fizz_scripted(t)); t_tick += stamp() - a; }
      show(&clear, !nosync, 1.0f);
    }
  } else {
    /* Interactive: the sim runs at 60 Hz from the sound clock (or the BIOS clock), the screen follows the retrace and
       draws between ticks. */
    unsigned long base = clock560(), done = 0;
    t = 0;
    for (;;) {
      unsigned long elapsed = clock560() - base, owed = elapsed * 60 / 560;
      if (bench ? t >= 600 : kbd_pressed(SC_ESC)) break;
      if (kbd_pressed(SC_P)) paused = !paused;
      if (kbd_pressed(SC_PGUP)) fizz_goto(fizz_level() + 15);
      if (kbd_pressed(SC_PGDN)) fizz_goto(fizz_level() + 1);
      if (kbd_pressed(SC_R)) fizz_goto(fizz_level());
      if (kbd_pressed(SC_N)) { night = !night; fizz_night(night); }
      if (kbd_pressed(SC_M)) snd_set_music(!snd_music_enabled());
      if (kbd_pressed(SC_E)) snd_set_fx(!snd_fx_enabled());
      if (owed > done + 6) { base = clock560(); done = 0; owed = 0; } /* after a stall, do not rush to catch up */
      if (owed > done) {
        /* The first tick of a frame also gets any quick taps; the rest of a catch-up see what is held. */
        unsigned bits = bench ? fizz_scripted(t) : kbd_tick_bits();
        while (owed > done) {
          if (!paused) { uclock_t a = stamp(); fizz_tick(bits); t_tick += stamp() - a; t++; }
          bits = bench ? fizz_scripted(t) : kbd_held();
          done++;
        }
        choose_music();
      }
      show(&clear, 1, paused ? 1.0f : (float)((elapsed * 60) % 560) / 560.0f);
    }
  }
  vga_close();
  kbd_remove();
  snd_shutdown();
  if (shot) {
    FILE *f = fopen("FRAME.RAW", "wb");
    if (f) { fwrite(frame, 1, 64000, f); fclose(f); }
    pal[765] = (clear >> 16) & 63; pal[766] = (clear >> 8) & 63; pal[767] = clear & 63;
    f = fopen("PAL.RAW", "wb");
    if (f) { fwrite(pal, 1, sizeof pal, f); fclose(f); }
  }
  printf("%u ticks, ended on level %u\n", t, fizz_level());
  if ((ticks && with_sound) || bench) printf("guest time %lu ms for %u frames\n", snd_ticks() * 1000UL / 560UL, ticks);
  if (n_frames)
    printf("per frame: build %lu us, paint %lu us (%u sprites), present %lu us; per tick: %lu us\n",
           (unsigned long)(t_build * 1000000ULL / UCLOCKS_PER_SEC / n_frames),
           (unsigned long)(t_draw * 1000000ULL / UCLOCKS_PER_SEC / n_frames), n_sprites / n_frames,
           (unsigned long)(t_present * 1000000ULL / UCLOCKS_PER_SEC / n_frames),
           t ? (unsigned long)(t_tick * 1000000ULL / UCLOCKS_PER_SEC / t) : 0UL);
  return 0;
}
