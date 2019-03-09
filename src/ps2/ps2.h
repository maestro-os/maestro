#ifndef PS2_H
# define PS2_H

# define PS2_DATA		0x60
# define PS2_STATUS		0x64
# define PS2_COMMAND	0x64

# define PS2_MAX_ATTEMPTS	3

# define KEYBOARD_ACK		0xfa
# define KEYBOARD_RESEND	0xf4

void disable_devices();
void enable_keyboard();

void ps2_init();

#endif
