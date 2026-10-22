/* AdLib (OPL2) music and effects and PC-speaker effects, driven by an IRQ 0 handler at 560 Hz.
 *
 * (c) Copyright 2026 Liminal HQ, Scott Morris
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

#include <go32.h>
#include <dpmi.h>
#include <pc.h>
#include <stdlib.h>
#include <string.h>
#include <dos.h>
#include "sound.h"

#define OPL_ADDR 0x388
#define OPL_DATA 0x389
#define TICK_HZ 560
#define PIT_DIVISOR 2131 /* 1193182 / 560 */

/* The whole of the handler's reach (opl_write to timer_end) is locked in memory and compiled in this order. */
typedef struct {
  const unsigned char *data;
  unsigned pos, end;
  long wait;
  int loop, on;
} Stream;

static const unsigned char *blob;
static unsigned long blob_length;
static unsigned n_songs, n_fx, n_captions, n_levels;
static const unsigned char *song_table, *fx_table, *caption_table, *level_table;
static int title_song, map_song, boss_song, n_extra;
static const unsigned char *extra_table;

static volatile unsigned long tick_count, register_writes;
static volatile int mode = SND_OFF;
static volatile int music_enabled = 1, fx_enabled = 1;
static volatile int have_opl, fx_dev = FXDEV_OFF, music_dev;
static volatile int want_music = -2, want_fx = -1; /* requests from the game, taken up by the handler */
static volatile int current_song = -1;
static volatile unsigned long music_t0; /* tick_count when the music stream last started */
static volatile int music_playing = -1; /* the song of that stream, -1 when none plays */
static Stream music, fx;
static const unsigned short *spk_data;
static volatile unsigned spk_pos, spk_frames;
static volatile int spk_on;
static volatile unsigned long old_acc;
static _go32_dpmi_seginfo old_isr, new_isr;
static int installed;

static inline __attribute__((always_inline)) unsigned rd16(const unsigned char *p) { return p[0] | (p[1] << 8); }
static inline __attribute__((always_inline)) unsigned long rd32(const unsigned char *p) { return rd16(p) | ((unsigned long)rd16(p + 2) << 16); }

static void opl_write(unsigned char reg, unsigned char val) {
  int i;
  outportb(OPL_ADDR, reg);
  for (i = 0; i < 6; i++) inportb(OPL_ADDR);
  outportb(OPL_DATA, val);
  for (i = 0; i < 35; i++) inportb(OPL_ADDR);
  register_writes++;
}

static void opl_silence(void) {
  int ch;
  for (ch = 0; ch < 9; ch++) opl_write(0xB0 + ch, 0);
  opl_write(0xBD, 0x20);
  opl_write(0xBD, 0);
}

static void stream_start(Stream *s, const unsigned char *chunk, int loop) {
  s->data = chunk;
  s->end = 2 + rd16(chunk);
  if (s->end < 6) { s->on = 0; return; }
  s->pos = 2;
  s->wait = 0;
  s->loop = loop;
  s->on = 1;
}

/* Runs one 560 Hz tick of a stream: every record whose time has come is written to the chip. */
static void stream_step(Stream *s) {
  if (!s->on) return;
  while (s->wait <= 0) {
    if (s->pos >= s->end) {
      if (!s->loop) { s->on = 0; return; }
      s->pos = 2;
    }
    opl_write(s->data[s->pos], s->data[s->pos + 1]);
    s->wait += rd16(s->data + s->pos + 2);
    s->pos += 4;
  }
  s->wait--;
}

static void speaker_off(void) { outportb(0x61, inportb(0x61) & 0xFC); }

static void speaker_set(unsigned divisor) {
  if (!divisor) { speaker_off(); return; }
  outportb(0x43, 0xB6);
  outportb(0x42, divisor & 255);
  outportb(0x42, divisor >> 8);
  outportb(0x61, inportb(0x61) | 3);
}

static void take_requests(void) {
  int m = want_music, f = want_fx;
  if (m != -2) {
    want_music = -2;
    if (have_opl) {
      opl_silence();
      music.on = 0;
      music_playing = -1;
      if (m >= 0 && (unsigned)m < n_songs && music_enabled && music_dev) {
        const unsigned char *e = song_table + m * 8;
        stream_start(&music, blob + rd32(e), 1);
        if (music.on) { music_playing = m; music_t0 = tick_count; }
      }
    }
  }
  if (f >= 0) {
    want_fx = -1;
    if (fx_enabled && (unsigned)f < n_fx) {
      const unsigned char *e = fx_table + f * 12;
      if (fx_dev == FXDEV_ADLIB && have_opl) {
        opl_write(0xB5, 0);
        stream_start(&fx, blob + rd32(e), 0);
      } else if (fx_dev == FXDEV_SPEAKER) {
        spk_data = (const unsigned short *)(blob + rd32(e + 6));
        spk_frames = rd16(e + 10);
        spk_pos = 0;
        spk_on = 1;
      }
    }
  }
}

static void call_old(void) { __asm__ volatile("pushfl; lcall *%0" : : "m"(old_isr.pm_offset) : "memory"); }

static void timer_isr(void) {
  tick_count++;
  take_requests();
  if (have_opl) {
    stream_step(&music);
    stream_step(&fx);
  }
  if (fx_dev == FXDEV_SPEAKER && spk_on && (tick_count & 3) == 0) {
    if (spk_pos >= spk_frames) { spk_on = 0; speaker_off(); }
    else speaker_set(spk_data[spk_pos++]);
  }
  /* DOS's own clock still wants its 18.2 Hz: pass every 30.75th tick on to the old handler, which also ends the interrupt. */
  old_acc += 18207;
  if (old_acc >= 559900UL) {
    old_acc -= 559900UL;
    call_old();
  } else {
    outportb(0x20, 0x20);
  }
}
static void timer_end(void) {}

static int opl_detect(void) {
  unsigned char s1, s2;
  opl_write(4, 0x60);
  opl_write(4, 0x80);
  s1 = inportb(OPL_ADDR);
  opl_write(2, 0xFF);
  opl_write(4, 0x21);
  delay(3);
  s2 = inportb(OPL_ADDR);
  opl_write(4, 0x60);
  opl_write(4, 0x80);
  return (s1 & 0xE0) == 0 && (s2 & 0xE0) == 0xC0;
}

int snd_detect_opl(void) { return opl_detect(); }

/* A Sound Blaster answers a reset of its DSP at port 0x220 with the byte 0xAA. */
int snd_detect_blaster(void) {
  int tries;
  outportb(0x226, 1);
  delay(1);
  outportb(0x226, 0);
  for (tries = 0; tries < 20; tries++) {
    delay(1);
    if ((inportb(0x22E) & 0x80) && inportb(0x22A) == 0xAA) return 1;
  }
  return 0;
}

static void set_pit(unsigned divisor) {
  outportb(0x43, 0x36);
  outportb(0x40, divisor & 255);
  outportb(0x40, (divisor >> 8) & 255);
}

int snd_init(const unsigned char *data, unsigned long length, int requested) {
  if (length < 9 || memcmp(data, "FZSN", 4) != 0) return SND_OFF;
  blob = data;
  blob_length = length;
  n_songs = data[5];
  n_fx = data[6];
  n_captions = data[7];
  n_levels = data[8];
  song_table = data + 9;
  fx_table = song_table + n_songs * 8;
  caption_table = fx_table + n_fx * 12;
  level_table = caption_table + n_captions;
  title_song = level_table[n_levels];
  map_song = level_table[n_levels + 1];
  boss_song = level_table[n_levels + 2];
  n_extra = level_table[n_levels + 3];
  extra_table = level_table + n_levels + 4;
  if (requested == SND_OFF) { mode = SND_OFF; return mode; }
  have_opl = requested != SND_SPEAKER && opl_detect();
  if (have_opl) { opl_silence(); opl_write(0x01, 0x20); }
  /* Which device plays what: AdLib everything, the music on AdLib and the effects on the speaker, or the speaker alone. */
  music_dev = have_opl;
  if (requested == SND_MIXED) fx_dev = FXDEV_SPEAKER;
  else if (requested == SND_SPEAKER) fx_dev = FXDEV_SPEAKER;
  else fx_dev = have_opl ? FXDEV_ADLIB : FXDEV_SPEAKER;
  if (requested == SND_ADLIB && !have_opl) { mode = SND_OFF; return mode; }
  mode = have_opl ? (fx_dev == FXDEV_SPEAKER ? SND_MIXED : SND_ADLIB) : SND_SPEAKER;
  /* What the handler touches stays in memory: the code, the streams and the data. */
  _go32_dpmi_lock_code((void *)opl_write, (unsigned long)timer_end - (unsigned long)opl_write);
  _go32_dpmi_lock_data((void *)&music, sizeof music);
  _go32_dpmi_lock_data((void *)&fx, sizeof fx);
  _go32_dpmi_lock_data((void *)blob, blob_length);
  _go32_dpmi_lock_data((void *)&tick_count, sizeof tick_count);
  _go32_dpmi_lock_data((void *)&music_t0, sizeof music_t0);
  _go32_dpmi_get_protected_mode_interrupt_vector(8, &old_isr);
  new_isr.pm_offset = (unsigned long)timer_isr;
  new_isr.pm_selector = _go32_my_cs();
  if (_go32_dpmi_allocate_iret_wrapper(&new_isr)) { mode = SND_OFF; return mode; }
  tick_count = 0;
  old_acc = 0;
  set_pit(PIT_DIVISOR);
  _go32_dpmi_set_protected_mode_interrupt_vector(8, &new_isr);
  installed = 1;
  atexit(snd_shutdown);
  return mode;
}

void snd_shutdown(void) {
  if (!installed) return;
  installed = 0;
  _go32_dpmi_set_protected_mode_interrupt_vector(8, &old_isr);
  set_pit(0);
  _go32_dpmi_free_iret_wrapper(&new_isr);
  if (have_opl) opl_silence();
  speaker_off();
  mode = SND_OFF;
}

int snd_mode(void) { return mode; }
unsigned long snd_ticks(void) { return tick_count; }
unsigned long snd_register_writes(void) { return register_writes; }
int snd_current_song(void) { return current_song; }

/* 560 Hz ticks since the playing song began, counting on through its loops; -1 while nothing plays on the OPL2 or a change of song is
   still waiting for the handler. The screens that follow the music (the opening titles) read it. */
long snd_music_clock(void) {
  long t = -1;
  if (!installed) return -1;
  disable();
  if (want_music == -2 && music.on && music_playing >= 0 && music_playing == current_song) t = (long)(tick_count - music_t0);
  enable();
  return t;
}

void snd_music(int song) {
  current_song = song;
  if (have_opl) want_music = song;
}

void snd_music_for(int level, int boss) {
  int song;
  if (!blob) return;
  if (boss) song = boss_song;
  else if (level == -2) song = title_song;
  else if (level < 0) song = map_song;
  else if ((unsigned)level < n_levels) song = level_table[level];
  else song = title_song;
  if (song != current_song) snd_music(song);
}

/* One of the extra songs the story screens ask for: 0 is the ending. */
void snd_music_extra(int n) {
  if (!blob || n < 0 || n >= n_extra) return;
  if (extra_table[n] != current_song) snd_music(extra_table[n]);
}

void snd_fx(int effect) { if (mode != SND_OFF && effect >= 0) want_fx = effect; }

void snd_caption(unsigned caption) {
  if (!blob || caption >= n_captions) return;
  if (caption_table[caption] != 255) snd_fx(caption_table[caption]);
}

void snd_set_music(int on) {
  music_enabled = on;
  if (!on) want_music = -1;
  else if (current_song >= 0) want_music = current_song;
}
void snd_set_fx(int on) { fx_enabled = on; }
int snd_music_enabled(void) { return music_enabled; }
int snd_have_opl(void) { return have_opl; }
int snd_fx_device(void) { return fx_dev; }
void snd_set_fx_device(int dev) {
  if (mode == SND_OFF) return;
  if (dev == FXDEV_ADLIB && !have_opl) dev = FXDEV_SPEAKER;
  fx_dev = dev;
  mode = have_opl ? (fx_dev == FXDEV_SPEAKER ? SND_MIXED : SND_ADLIB) : SND_SPEAKER;
}
int snd_fx_enabled(void) { return fx_enabled; }
