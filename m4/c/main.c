/* Episode 1 on a VGA mode 13h screen, with sound.
   usage: FIZZ [level|map] [ticks] [shot] [nosync] [night] [nosound] [adlib] [mixed] [speaker] [soundtest]
   With no ticks (or 0) it is interactive: arrows or WASD move, Ctrl or Z jump, Alt or X pogo, Space or C fire,
   PgUp and PgDn change level, R restarts it, Backspace goes back to the map, N toggles night, M music, E effects, Esc pauses
   (and goes back in a menu; Quit to title, then Quit, leaves).
   With ticks it runs a scripted walk for that many ticks and, with "shot", writes FRAME.RAW (64000 bytes) and PAL.RAW
   (256 x RGB, 6-bit) at the end. "soundtest" plays a song for a few seconds headless and reports what the driver did. */
#include <conio.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include "compat.h"
#include "kbd.h"
#include "keymap.h"
#include "save.h"
#include "sound.h"
#include "vga.h"

extern void fizz_start(unsigned level, int use53);
extern unsigned fizz_scripted(unsigned tick);
extern void fizz_tick(unsigned held);
extern void fizz_goto(unsigned level);
extern unsigned fizz_level(void);
extern unsigned fizz_boss_hp(void);
extern unsigned fizz_mode(void);
extern void fizz_press(int on);
extern unsigned fizz_quit(void);
extern void fizz_show(unsigned n);
extern void fizz_goto_map(void);
extern void fizz_night(int on);
extern unsigned fizz_build(float alpha);
extern unsigned fizz_paint(unsigned char *frame, unsigned count);
extern void fizz_preview(unsigned char *frame, const int *values, const char *cpu);
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

/* The frame rate setting: auto, 60 Hz or 30 Hz. At 30 the screen is drawn on every second tick. Auto picks one from what the
   frames cost (see the main loop). */
static int rate = 0, rate_now = 60;
int rate_mode(void) { return rate; }
void rate_set(int mode) { rate = mode; if (mode) rate_now = mode == 2 ? 30 : 60; }
int rate_active(void) { return rate_now; }

/* The detail setting: auto, high or low. Low leaves out the backdrop layers (see video.rs). Auto goes low when frames cost more than
   45 ms and back to high when low frames cost under 17 ms. */
extern void fizz_detail(int low);
static int detail = 0, detail_low = 0;
int detail_mode(void) { return detail; }
int detail_active(void) { return detail_low; }
static void detail_apply(int low) { detail_low = low; fizz_detail(low); }
void detail_set(int mode) { detail = mode; if (mode) detail_apply(mode == 2); }

/* The options, kept in FIZZ.CFG beside the game: twelve bytes, "FZC", a version, then music, effects, the effects' device, frame rate,
   detail, night, the sound choice and a spare. The options screen saves after each change; the start-up screen saves the sound
   choice. Arguments on the command line override what was kept, for that run. */
static int sound_pref = SND_AUTO, night_pref = 0, music_pref = 1, fx_pref = 1, dev_pref = 0;

static void settings_load(void) {
  unsigned char b[12];
  if (save_read(-1, b, 12) != 12 || b[0] != 'F' || b[1] != 'Z' || b[2] != 'C' || b[3] != 1) return;
  music_pref = b[4] != 0;
  fx_pref = b[5] != 0;
  dev_pref = b[6];
  rate = b[7] > 2 ? 0 : b[7];
  detail = b[8] > 2 ? 0 : b[8];
  night_pref = b[9] != 0;
  sound_pref = b[10] > SND_MIXED ? SND_AUTO : b[10];
  if (rate) rate_now = rate == 2 ? 30 : 60;
  if (detail) detail_low = detail == 2;
}

/* Called by the options screen with the night setting, which lives in the game. */
void settings_save(int night) {
  unsigned char b[12] = {'F', 'Z', 'C', 1};
  night_pref = night;
  b[4] = (unsigned char)snd_music_enabled();
  b[5] = (unsigned char)snd_fx_enabled();
  b[6] = (unsigned char)(snd_fx_device() ? snd_fx_device() : dev_pref); /* with the sound off there is no device to read */
  b[7] = (unsigned char)rate;
  b[8] = (unsigned char)detail;
  b[9] = (unsigned char)night;
  b[10] = (unsigned char)sound_pref;
  save_write(-1, b, 12);
}

/* The game's clock in 1/560 s: the sound handler's count when it is running, else the BIOS clock. */
static unsigned long clock560(void) {
  if (snd_mode() != SND_OFF) return snd_ticks();
  return (unsigned long)((uclock() * 560ULL) / UCLOCKS_PER_SEC);
}

/* Frames a second of the real renderer: build and paint a few frames of the current scene and time them. The timer is still
   the BIOS's here, so uclock() can be trusted. */
static int speed_test(void) {
  int i;
  unsigned count;
  uclock_t a, us;
  for (i = 0; i < 3; i++) { count = fizz_build(1.0f); fizz_paint(frame, count); }
  a = uclock();
  for (i = 0; i < 30; i++) { count = fizz_build(1.0f); fizz_paint(frame, count); }
  us = (uclock() - a) * 1000000ULL / UCLOCKS_PER_SEC;
  return us ? (int)(30ULL * 1000000ULL / us) : 0;
}

/* What the sound option will really be on this machine. */
static int sound_using(int choice, int adlib) {
  if (choice == SND_OFF) return 3;
  if (choice == SND_SPEAKER) return 2;
  if (choice == SND_MIXED) return adlib ? 4 : 2;
  if (adlib) return 1;
  return choice == SND_AUTO ? 2 : 3;
}

static void choose_music(void) {
  static int last_level = -2, last_boss = -2;
  int level = fizz_mode() == 1 ? -2 : fizz_mode() == 2 ? -1 : (int)fizz_level(), boss = fizz_mode() == 3 && fizz_boss_hp() > 0;
  if (level != last_level || boss != last_boss) {
    last_level = level;
    last_boss = boss;
    snd_music_for(level, boss);
  }
}

int main(int argc, char **argv) {
  /* The first argument is a level number, or "map" for the overworld (254 to the game), or nothing for the title (255). */
  unsigned level = argc < 2 ? 255 : !strcmp(argv[1], "map") ? 254 : (unsigned)atoi(argv[1]);
  unsigned ticks = argc > 2 ? (unsigned)atoi(argv[2]) : 0;
  int shot = 0, nosync = 0, i, paused = 0, night = 0, sound_mode = SND_AUTO, soundtest = 0, with_sound = 0, quiet = 0, bench = 0, preview = 1, previewshot = 0, screen = 0, savecfg = 0;
  int probe[C_COUNT];
  char cpu[16];
  unsigned t, clear = 0;
  unsigned char pal[768];
  settings_load();
  sound_mode = sound_pref;
  night = night_pref;
  for (i = 3; i < argc; i++) {
    if (!strcmp(argv[i], "shot")) shot = 1;
    if (!strcmp(argv[i], "nosync")) nosync = 1;
    if (!strcmp(argv[i], "night")) night = 1;
    if (!strcmp(argv[i], "nosound")) sound_mode = SND_OFF;
    if (!strcmp(argv[i], "adlib")) sound_mode = SND_ADLIB;
    if (!strcmp(argv[i], "speaker")) sound_mode = SND_SPEAKER;
    if (!strcmp(argv[i], "mixed")) sound_mode = SND_MIXED;
    if (!strcmp(argv[i], "soundtest")) soundtest = 1;
    if (!strcmp(argv[i], "sound")) with_sound = 1;
    if (!strcmp(argv[i], "quiet")) quiet = 1;
    if (!strcmp(argv[i], "bench")) bench = 1;
    if (!strcmp(argv[i], "press")) fizz_press(1);
    if (!strcmp(argv[i], "approach")) fizz_press(2);
    if (!strcmp(argv[i], "noinput")) fizz_press(3);
    if (!strcmp(argv[i], "nopreview")) preview = 0;
    if (!strcmp(argv[i], "pause")) screen = 1;
    if (!strcmp(argv[i], "options")) screen = 2;
    if (!strcmp(argv[i], "load")) screen = 3;
    if (!strcmp(argv[i], "save")) screen = 4;
    if (!strcmp(argv[i], "dialogue")) screen = 5;
    if (!strncmp(argv[i], "loadslot", 8)) screen = 10 + atoi(argv[i] + 8);
    if (!strcmp(argv[i], "preview")) previewshot = 1;
    if (!strcmp(argv[i], "fps60")) rate_set(1);
    if (!strcmp(argv[i], "fps30")) rate_set(2);
    if (!strcmp(argv[i], "fpsauto")) rate_set(0);
    if (!strcmp(argv[i], "lowdetail")) detail_set(2);
    if (!strcmp(argv[i], "highdetail")) detail_set(1);
    if (!strcmp(argv[i], "autodetail")) detail_set(0);
    if (!strcmp(argv[i], "savecfg")) savecfg = 1;
  }
  if (savecfg) { settings_save(night); printf("saved FIZZ.CFG\n"); return 0; }
  fizz_start(level, 1);
  fizz_night(night);
  if (screen) fizz_show(screen);

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

  if (ticks == 0 && !bench) {
    if (kbd_install()) { printf("could not hook the keyboard\n"); snd_shutdown(); return 1; }
  }
  if (vga_open()) { printf("no near pointers\n"); kbd_remove(); snd_shutdown(); return 1; }
  build_palette(pal);
  for (i = 0; i < 255; i++) vga_palette(i, pal[i * 3], pal[i * 3 + 1], pal[i * 3 + 2]);

  /* The start-up screen: what the machine has, a speed test, and a chance to pick the sound. It runs before the sound handler
     takes over the timer. */
  if ((ticks == 0 && !bench && preview) || previewshot) {
    compat_probe(probe, cpu);
    probe[C_FPS] = speed_test();
    for (;;) {
      probe[C_SOUND_CHOICE] = sound_mode;
      probe[C_SOUND_USING] = sound_using(sound_mode, probe[C_ADLIB]);
      fizz_preview(frame, probe, cpu);
      vga_vsync();
      vga_present(frame);
      if (previewshot) break;
      if (kbd_pressed(SC_ENTER) || kbd_pressed(SC_SPACE)) { sound_pref = sound_mode; settings_save(night); break; }
      if (kbd_pressed(SC_ESC)) { vga_close(); kbd_remove(); return 0; }
      if (kbd_pressed(SC_A)) sound_mode = SND_ADLIB;
      if (kbd_pressed(SC_M)) sound_mode = SND_MIXED;
      if (kbd_pressed(SC_P)) sound_mode = SND_SPEAKER;
      if (kbd_pressed(SC_O)) sound_mode = SND_OFF;
    }
  }
  if (!previewshot && ticks == 0 && !bench && preview) {
    /* Enter ended the start-up screen; let it go before the menu looks, or the menu would take it as its own press. */
    while (kbd_down(SC_ENTER) || kbd_down(SC_SPACE)) {}
    kbd_pressed(SC_ENTER);
    kbd_pressed(SC_SPACE);
    kbd_tick_bits();
  }
  if (!previewshot && (ticks == 0 || with_sound)) {
    if (sound_mode != SND_OFF) {
      snd_init(fizz_sound_data(), fizz_sound_len(), sound_mode);
      snd_set_music(music_pref);
      snd_set_fx(fx_pref);
      if (dev_pref) snd_set_fx_device(dev_pref);
    }
    if ((ticks || bench) && !quiet) snd_music(0);
  }

  if (previewshot) {
    /* Nothing more to run: the frame is the screenshot. */
  } else if (ticks) {
    /* A scripted run: one tick a frame, so a screenshot is the same every time. */
    for (t = 0; t < ticks; t++) {
      { uclock_t a = stamp(); fizz_tick(fizz_scripted(t)); t_tick += stamp() - a; }
      show(&clear, !nosync, 1.0f);
    }
  } else {
    /* Interactive: the sim runs at 60 Hz from the sound clock (or the BIOS clock), the screen follows the retrace and
       draws between ticks. */
    unsigned long base = clock560(), done = 0, spent = 0;
    unsigned drawn = 0, frames = 0;
    t = 0;
    for (;;) {
      unsigned long elapsed = clock560() - base, owed = elapsed * 60 / 560;
      if (bench ? t >= 600 : fizz_quit()) break;
      if (kbd_pressed(SC_PGUP)) fizz_goto(fizz_level() + 15);
      if (kbd_pressed(SC_PGDN)) fizz_goto(fizz_level() + 1);
      if (kbd_pressed(SC_R)) fizz_goto(fizz_level());
      if (kbd_pressed(SC_BACKSPACE)) fizz_goto_map();
      if (kbd_pressed(SC_N)) { night = !night; fizz_night(night); }
      if (kbd_pressed(SC_M)) snd_set_music(!snd_music_enabled());
      if (kbd_pressed(SC_E)) snd_set_fx(!snd_fx_enabled());
      if (owed > done + 6) { base = clock560(); done = 0; owed = 0; } /* after a stall, do not rush to catch up */
      if (owed > done) {
        /* The first tick of a frame also gets any quick taps; the rest of a catch-up see what is held. */
        unsigned bits = bench ? fizz_scripted(t) : kbd_tick_bits();
        while (owed > done) {
          { uclock_t a = stamp(); fizz_tick(bits); t_tick += stamp() - a; t++; }
          bits = bench ? fizz_scripted(t) : kbd_held();
          done++;
        }
        choose_music();
      }
      if (rate_now == 30 && t < drawn + 2) continue; /* 30 Hz: every second tick */
      {
        unsigned long a = clock560();
        show(&clear, 1, (float)((elapsed * 60) % 560) / 560.0f);
        drawn = t;
        if (rate == 0) {
          /* The average cost of a frame over 32, in 1/560 s: a sixtieth of a second is 9.3. */
          spent += clock560() - a;
          if (++frames == 32) {
            /* A frame's cost against a sixtieth of a second, in percent. Between that and about 1.7 times it, 30 Hz keeps the pace
               steady; dearer than that a cap would only throw frames away, so the screen is drawn as often as it can be. */
            unsigned long pct = spent * 60 * 100 / (32UL * 560);
            if (detail == 0) {
              unsigned long ms = spent * 1000 / (32UL * 560); /* a frame's average cost in milliseconds */
              if (!detail_low && ms > 45) detail_apply(1);
              else if (detail_low && ms < 17) detail_apply(0);
            }
            if (pct > 100 && pct <= 170) rate_now = 30;
            else if (pct > 185 || pct < 67) rate_now = 60;
            spent = 0;
            frames = 0;
          }
        }
      }
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
  printf("%u ticks, ended in mode %u (2 map, 3 level) on level %u\n", t, fizz_mode(), fizz_level());
  if ((ticks && with_sound) || bench) printf("guest time %lu ms for %u frames\n", snd_ticks() * 1000UL / 560UL, ticks);
  if (n_frames) printf("%u frames drawn for %u ticks, rate in use %d Hz, detail %s\n", n_frames, t, rate_now, detail_low ? "low" : "high");
  if (n_frames)
    printf("per frame: build %lu us, paint %lu us (%u sprites), present %lu us; per tick: %lu us\n",
           (unsigned long)(t_build * 1000000ULL / UCLOCKS_PER_SEC / n_frames),
           (unsigned long)(t_draw * 1000000ULL / UCLOCKS_PER_SEC / n_frames), n_sprites / n_frames,
           (unsigned long)(t_present * 1000000ULL / UCLOCKS_PER_SEC / n_frames),
           t ? (unsigned long)(t_tick * 1000000ULL / UCLOCKS_PER_SEC / t) : 0UL);
  return 0;
}
