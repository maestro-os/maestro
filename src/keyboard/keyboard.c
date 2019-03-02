#include "../kernel.h"
#include "../idt/idt.h"
#include "keyboard.h"

#include "../libc/stdio.h"

static inline uint8_t get_config_byte()
{
	outb(PS2_COMMAND, 0x20);
	return inb(0x60);
}

static inline void set_config_byte(const uint8_t config_byte)
{
	outb(PS2_COMMAND, 0x60);
	outb(PS2_DATA, config_byte);
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

void keyboard_init()
{
	outb(PS2_COMMAND, 0xad);
	outb(PS2_COMMAND, 0xa7);

	inb(PS2_DATA);

	set_config_byte(get_config_byte() & 0b10111100);

	if(!test_controller()) return;
	if(!test_device()) return;

	outb(PS2_COMMAND, 0xae);

	set_config_byte(get_config_byte() | 0b00000001);

	outb(PS2_COMMAND, 0xff);

	if(inb(PS2_DATA) == 0xfc)
	{
		printf("Failed to reset PS/2 controller D:\n");
		return;
	}

	printf("PS/2 reset: OK :D\n");

	set_interrupts_state(1);
}
