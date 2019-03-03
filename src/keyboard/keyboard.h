#ifndef KEYBOARD_H
# define KEYBOARD_H

# define PS2_DATA		0x60
# define PS2_STATUS		0x64
# define PS2_COMMAND	0x64

# define KEYBOARD_ACK		0xfa
# define KEYBOARD_RESEND	0xf4

void keyboard_init();

#endif
