#ifndef COMPAT_H
#define COMPAT_H
/* What the machine has, for the start-up screen. The numbers go in the same slots as the Rust side's `preview` constants. */
enum { C_VGA, C_EGA, C_MOUSE, C_JOYSTICK, C_ADLIB, C_BLASTER, C_FPU, C_MEMORY_KB, C_FPS, C_SOUND_CHOICE, C_SOUND_USING, C_UNUSED, C_COUNT };

/* Looks at the hardware (before the sound handler takes the timer). `cpu` receives the processor's name. */
void compat_probe(int v[C_COUNT], char cpu[16]);
#endif
