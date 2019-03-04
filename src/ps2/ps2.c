#include "../kernel.h"
#include "../idt/idt.h"
#include "keyboard.h"

#include "../libc/stdio.h"

uint8_t keyboard_command(const uint8_t command)
{
	uint8_t response;
	uint8_t attempts = 0;

	do
	{
		outb(PS2_COMMAND, command);
	}
	while((response = inb(PS2_DATA)) == KEYBOARD_RESEND && ++attempts < 3);

	return response;
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
	outb(PS2_DATA, config_byte);
	outb(PS2_COMMAND, 0x60);
}

int test_controller()
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

int test_device()
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

int ps2_init()
{
	disable_devices();

	inb(PS2_DATA);

	set_config_byte(get_config_byte() & 0b10111100);

	return (test_controller() && test_device());
}

void keyboard_init()
{
	if(!ps2_init()) return;

	enable_keyboard();

	outb(PS2_DATA, 0b00011111);
	keyboard_command(0xf3);

	// TODO
	if(keyboard_command(0xf4) != KEYBOARD_ACK)
	{
		printf("Failed to enable keyboard! D:\n");
	}
	else
	{
		printf("Keyboard enabled! :D\n");
	}

	set_config_byte(get_config_byte() | 0b00000001);
	set_interrupts_state(1);
}
