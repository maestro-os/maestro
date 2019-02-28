#include "../kernel.h"
#include "pic.h"

void pic_init()
{
	outb(PIC_MASTER_COMMAND, PIC_COMMAND_INIT);
	outb(PIC_SLAVE_COMMAND, PIC_COMMAND_INIT);
}

void pic_EOI(const uint8_t irq)
{
	if(irq >= 0x8) outb(PIC_SLAVE_COMMAND, PIC_COMMAND_EOI);
	outb(PIC_MASTER_COMMAND, PIC_COMMAND_EOI);
}
