#ifndef KBD_H
#define KBD_H
int kbd_install(void);   /* hook IRQ 1; 0 on success */
void kbd_remove(void);   /* give the keyboard back to DOS (also run at exit) */
unsigned kbd_held(void); /* the game's input bits right now */
int kbd_down(int scancode);
/* The game bits for one tick: what is held, plus any key pressed and released since the last tick (so a quick tap is never lost). */
unsigned kbd_tick_bits(void);
/* 1 once for each press of the key since the last call. */
int kbd_pressed(int scancode);
#endif
