#include "kernel.h"
#include "ps2/ps2.h"
#include "idt/idt.h"

static void in_enable_a20()
{
	disable_devices();

	outb(PS2_COMMAND, 0xd0);
	const uint8_t in = inb(PS2_DATA);

	outb(PS2_COMMAND, 0xd1);
	outb(PS2_DATA, in | 0b10);

	enable_keyboard();
}

void enable_a20()
{
	idt_setup_wrap(in_enable_a20);
}
