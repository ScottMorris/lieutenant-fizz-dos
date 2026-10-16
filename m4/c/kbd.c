/* The keyboard through IRQ 1 (interrupt 9): every make and break code from port 0x60 is recorded, so the game sees
   which keys are held, as it cannot from DOS's key buffer. */
#include <go32.h>
#include <dpmi.h>
#include <pc.h>
#include <stdlib.h>
#include <string.h>
#include "kbd.h"
#include "keymap.h"

static volatile unsigned char keys[128];
static volatile unsigned char latch[128];
static _go32_dpmi_seginfo old_isr, new_isr;
static int installed;

static void isr(void) {
  unsigned char sc = inportb(0x60);
  /* 0xE0 and 0xE1 prefix the grey keys and Pause; the code that follows is what we record. */
  if (sc != 0xE0 && sc != 0xE1) {
    unsigned char code = sc & 0x7F;
    if (sc & 0x80) {
      keys[code] = 0;
    } else {
      if (!keys[code]) latch[code] = 1;
      keys[code] = 1;
    }
  }
  outportb(0x20, 0x20); /* end of interrupt for the interrupt controller */
}
static void isr_end(void) {}

int kbd_install(void) {
  if (installed) return 0;
  memset((void *)keys, 0, sizeof keys);
  memset((void *)latch, 0, sizeof latch);
  /* The handler and what it touches must never be paged out, or the first key during a page fault would hang. */
  if (_go32_dpmi_lock_data((void *)keys, sizeof keys) || _go32_dpmi_lock_data((void *)latch, sizeof latch) ||
      _go32_dpmi_lock_code((void *)isr, (unsigned long)isr_end - (unsigned long)isr))
    return 1;
  _go32_dpmi_get_protected_mode_interrupt_vector(9, &old_isr);
  new_isr.pm_offset = (unsigned long)isr;
  new_isr.pm_selector = _go32_my_cs();
  if (_go32_dpmi_allocate_iret_wrapper(&new_isr)) return 1;
  if (_go32_dpmi_set_protected_mode_interrupt_vector(9, &new_isr)) return 1;
  installed = 1;
  atexit(kbd_remove);
  return 0;
}

void kbd_remove(void) {
  if (!installed) return;
  _go32_dpmi_set_protected_mode_interrupt_vector(9, &old_isr);
  _go32_dpmi_free_iret_wrapper(&new_isr);
  installed = 0;
}

unsigned kbd_held(void) { return keymap_bits(keys); }

unsigned kbd_tick_bits(void) {
  /* Keys of the game only: a press of Esc, P, R or the page keys is for the main loop, which reads it by itself. */
  static const unsigned char game_keys[] = {SC_LEFT, SC_RIGHT, SC_UP, SC_DOWN, SC_A, SC_D, SC_W, SC_S,
                                            SC_CTRL, SC_Z, SC_ALT, SC_X, SC_SPACE, SC_C, SC_ENTER, SC_ESC};
  unsigned char taps[128];
  unsigned i;
  memset(taps, 0, sizeof taps);
  for (i = 0; i < sizeof game_keys; i++) {
    taps[game_keys[i]] = latch[game_keys[i]];
    latch[game_keys[i]] = 0;
  }
  return keymap_bits(keys) | keymap_bits(taps);
}
int kbd_down(int sc) { return keys[sc & 0x7F]; }
int kbd_pressed(int sc) {
  int was = latch[sc & 0x7F];
  latch[sc & 0x7F] = 0;
  return was;
}
