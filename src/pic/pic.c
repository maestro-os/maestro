#include <pic/pic.h>

__attribute__((cold))
void pic_init(const uint8_t offset1, const uint8_t offset2)
{
	int8_t mask1, mask2;

	mask1 = inb(PIC_MASTER_DATA);
	mask2 = inb(PIC_SLAVE_DATA);

	// TODO io_wait?
	outb(PIC_MASTER_COMMAND, ICW1_INIT | ICW1_ICW4);
	outb(PIC_SLAVE_COMMAND, ICW1_INIT | ICW1_ICW4);

	outb(PIC_MASTER_DATA, offset1);
	outb(PIC_SLAVE_DATA, offset2);

	outb(PIC_MASTER_DATA, ICW3_SLAVE_PIC);
	outb(PIC_SLAVE_DATA, ICW3_CASCADE);

	outb(PIC_MASTER_DATA, ICW4_8086);
	outb(PIC_SLAVE_DATA, ICW4_8086);

	outb(PIC_MASTER_DATA, mask1);
	outb(PIC_SLAVE_DATA, mask2);
}

__attribute__((hot))
void pic_EOI(const unsigned irq)
{
	if(irq >= 0x8)
		outb(PIC_SLAVE_COMMAND, PIC_COMMAND_EOI);
	outb(PIC_MASTER_COMMAND, PIC_COMMAND_EOI);
}
