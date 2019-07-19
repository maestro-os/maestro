#include <memory/memory.h>
#include <idt/idt.h>
#include <ps2/ps2.h>

void enable_a20(void)
{
	idt_set_state(false);
	ps2_disable_devices();

	outb(PS2_COMMAND, 0xd0);
	const uint8_t in = inb(PS2_DATA);

	outb(PS2_COMMAND, 0xd1);
	outb(PS2_DATA, in | 0b10);
}
