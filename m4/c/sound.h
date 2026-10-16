#ifndef SOUND_H
#define SOUND_H
/* Music and sound effects for the DOS build: an OPL2 (AdLib) player for IMF streams and a PC-speaker player for
   effects, both driven from IRQ 0 reprogrammed to 560 Hz, the IMF clock. */
enum { SND_AUTO = 0, SND_ADLIB = 1, SND_SPEAKER = 2, SND_OFF = 3 };

/* Loads the sound data and, for SND_AUTO, looks for an OPL2 card (falling back to the PC speaker). Installs the timer
   handler. Returns the mode chosen. */
int snd_init(const unsigned char *blob, unsigned long length, int mode);
void snd_shutdown(void);
int snd_mode(void);

/* The 560 Hz tick count since snd_init: the game's clock while the handler is installed. */
unsigned long snd_ticks(void);

void snd_music(int song);        /* -1 stops */
void snd_music_for(int level, int boss);
int snd_current_song(void);
void snd_fx(int effect);
void snd_caption(unsigned caption); /* the sim's caption id: plays its effect */

void snd_set_music(int on);
void snd_set_fx(int on);
int snd_music_enabled(void);
int snd_fx_enabled(void);

/* For checks. */
unsigned long snd_register_writes(void);
#endif
