#include "../kernel.h"
#include "ps2.h"
#include "../idt/idt.h"

#include "../libc/stdio.h"

static uint8_t keyboard_command(const uint8_t command)
{
	uint8_t response;
	uint8_t attempts = 0;

	do
	{
		outb(PS2_DATA, command);
	}
	while((response = inb(PS2_DATA)) == KEYBOARD_RESEND && ++attempts < 3);

	return response;
}

static uint8_t keyboard_command_data(const uint8_t command, const uint8_t data)
{
	uint8_t response;
	uint8_t attempts = 0;

	do
	{
		outb(PS2_DATA, command);
		outb(PS2_DATA, data);
	}
	while((response = inb(PS2_DATA)) == KEYBOARD_RESEND && ++attempts < 3);

	return response;
}

static int test_controller()
{
	outb(PS2_COMMAND, 0xaa);

	if(inb(PS2_DATA) == 0x55)
	{
		printf("PS/2 controller: OK :D\n");
		return 1;
	}
	else
	{
		printf("PS/2 controller: KO D:\n");
		return 0;
	}
}

static int test_device()
{
	outb(PS2_COMMAND, 0xab);

	if(inb(PS2_DATA) == 0x00)
	{
		printf("PS/2 first device: OK :D\n");
		return 1;
	}
	else
	{
		printf("PS/2 first device: KO D:\n");
		return 0;
	}
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

uint8_t get_config_byte()
{
	outb(PS2_COMMAND, 0x20);
	return inb(PS2_DATA);
}

void set_config_byte(const uint8_t config_byte)
{
	outb(PS2_COMMAND, 0x60);
	outb(PS2_DATA, config_byte);
}

void ps2_init()
{
	set_interrupts_state(0);

	// TODO Check if existing using ACPI
	disable_devices();

	inb(PS2_DATA);

	set_config_byte(get_config_byte() & 0b10111100);

	if(test_controller() && test_device())
	{
		enable_keyboard();

		// TODO
		if(keyboard_command_data(0xf3, 0b00000000) == KEYBOARD_ACK
			&& keyboard_command(0xf4) == KEYBOARD_ACK)
		{
			printf("Keyboard enabled! :D\n");
		}
		else
		{
			printf("Failed to enable keyboard! D:\n");
			return;
		}

		set_config_byte(get_config_byte() | 0b00000001);

		if(test_controller())
		{
			test_device();
		}
	}

	set_interrupts_state(1);
}
