#include "ps2.h"

// TODO Timeout
static uint8_t keyboard_command(const uint8_t command)
{
	uint8_t response;
	uint8_t attempts = 0;

	do
		outb(PS2_DATA, command);
	while((response = inb(PS2_DATA)) == KEYBOARD_RESEND
		&& ++attempts < PS2_MAX_ATTEMPTS);

	return response;
}

// TODO Timeout
static uint8_t keyboard_command_data(const uint8_t command, const uint8_t data)
{
	uint8_t response;
	uint8_t attempts = 0;

	do
	{
		outb(PS2_DATA, command);
		outb(PS2_DATA, data);
	}
	while((response = inb(PS2_DATA)) == KEYBOARD_RESEND
		&& ++attempts < PS2_MAX_ATTEMPTS);

	return response;
}

static inline bool test_controller()
{
	outb(PS2_COMMAND, 0xaa);
	return (inb(PS2_DATA) == 0x55);
}

static inline bool test_device()
{
	outb(PS2_COMMAND, 0xab);
	return (inb(PS2_DATA) == 0x00);
}

void disable_devices()
{
	outb(PS2_COMMAND, 0xad);
	outb(PS2_COMMAND, 0xa7);
}

void enable_keyboard()
{
	outb(PS2_COMMAND, 0xae);
}

static inline uint8_t get_config_byte()
{
	outb(PS2_COMMAND, 0x20);
	return inb(PS2_DATA);
}

static inline void set_config_byte(const uint8_t config_byte)
{
	outb(PS2_COMMAND, 0x60);
	outb(PS2_DATA, config_byte);
}

static void in_ps2_init()
{
	// TODO Check if existing using ACPI
	disable_devices();

	inb(PS2_DATA);

	set_config_byte(get_config_byte() & 0b10111100);

	if(!test_controller())
	{
		printf("PS/2 controller: KO D:\n");
		return;
	}

	if(keyboard_command(0xf4) != KEYBOARD_ACK)
	{
		printf("Failed to enable keyboard!\n");
		return;
	}

	// TODO
	(void)keyboard_command_data;

	if(!test_device())
	{
		printf("PS/2 first device: KO D:\n");
		return;
	}
}

void ps2_init()
{
	idt_setup_wrap(in_ps2_init);
}
