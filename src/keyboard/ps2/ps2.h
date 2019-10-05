#ifndef PS2_H
# define PS2_H

# include <kernel.h>
# include <idt/idt.h>
# include <libc/stdio.h>

# define PS2_DATA		0x60
# define PS2_STATUS		0x64
# define PS2_COMMAND	0x64

# define PS2_MAX_ATTEMPTS	3

# define CONTROLLER_TEST_PASS	0x55
# define CONTROLLER_TEST_FAIL	0xfc

# define KEYBOARD_ACK		0xfa
# define KEYBOARD_RESEND	0xf4

# define KEYBOARD_TEST_PASS	0x00
# define CLK_LINE_STK_LOW	0x01
# define CLK_LINE_STK_HIGH	0x02
# define DATA_LINE_STK_LOW	0x03
# define DATA_LINE_STK_HIGH	0x04

# define LED_SCROLL_LOCK	0b001
# define LED_NUMBER_LOCK	0b010
# define LED_CAPS_LOCK		0b100

void ps2_disable_devices(void);
bool ps2_enable_keyboard(void);

void ps2_init(void);
void ps2_set_keyboard_hook(void (*hook)(const uint8_t));
int8_t ps2_get_leds_state(void);
void ps2_set_leds_state(const int8_t state);

void ps2_keyboard_event(void);

#endif
