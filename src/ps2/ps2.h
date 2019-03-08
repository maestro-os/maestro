#ifndef PS2_H
# define PS2_H

# define PS2_DATA		0x60
# define PS2_STATUS		0x64
# define PS2_COMMAND	0x64

# define KEYBOARD_ACK		0xfa
# define KEYBOARD_RESEND	0xf4

void disable_devices();
void enable_keyboard();

uint8_t get_config_byte();
void set_config_byte(const uint8_t config_byte);

void ps2_init();

#endif
