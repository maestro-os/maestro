#include "../kernel.h"
#include "../pic/pic.h"
#include "idt.h"

interrupt_descriptor_t id[286];

static void remap_PIC()
{
	// TODO Detect if APIC is present

	unsigned char master_mask, slave_mask;
	master_mask = inb(PIC_MASTER_DATA);
	slave_mask = inb(PIC_SLAVE_DATA);

	pic_init();
	outb(PIC_MASTER_DATA, 0x20);
	outb(PIC_SLAVE_DATA, 0x28);
	outb(PIC_MASTER_DATA, 4);
	outb(PIC_SLAVE_DATA, 2);

	outb(PIC_MASTER_DATA, 0x1);
	outb(PIC_SLAVE_DATA, 0x1);

	outb(PIC_MASTER_DATA, master_mask);
	outb(PIC_SLAVE_DATA, slave_mask);
}

static interrupt_descriptor_t create_id(const void *address,
	const uint16_t selector, const uint8_t type_attr)
{
	interrupt_descriptor_t id;
	id.offset = ((unsigned long) address) & 0xffff;
	id.selector = selector;
	//id.zero = 0;
	id.type_attr = type_attr;
	id.offset_2 = (((unsigned long) address) & 0xffff0000) >> sizeof(id.offset);

	return id;
}

#include "../libc/stdio.h"

void idt_init()
{
	remap_PIC();

	// TODO Fix macros
	bzero(id, sizeof(id));
	id[32] = create_id(irq0, 0x8, 0x8e); // TODO Selector
	id[33] = create_id(irq1, 0x8, 0x8e); // TODO Selector
	id[34] = create_id(irq2, 0x8, 0x8e); // TODO Selector
	id[35] = create_id(irq3, 0x8, 0x8e); // TODO Selector
	id[36] = create_id(irq4, 0x8, 0x8e); // TODO Selector
	id[37] = create_id(irq5, 0x8, 0x8e); // TODO Selector
	id[38] = create_id(irq6, 0x8, 0x8e); // TODO Selector
	id[39] = create_id(irq7, 0x8, 0x8e); // TODO Selector
	id[40] = create_id(irq8, 0x8, 0x8e); // TODO Selector
	id[41] = create_id(irq9, 0x8, 0x8e); // TODO Selector
	id[42] = create_id(irq10, 0x8, 0x8e); // TODO Selector
	id[43] = create_id(irq11, 0x8, 0x8e); // TODO Selector
	id[44] = create_id(irq12, 0x8, 0x8e); // TODO Selector
	id[45] = create_id(irq13, 0x8, 0x8e); // TODO Selector
	id[46] = create_id(irq14, 0x8, 0x8e); // TODO Selector
	id[47] = create_id(irq15, 0x8, 0x8e); // TODO Selector

	idt_t idt;
	idt.limit = sizeof(id) - 1;
	idt.base = (uint32_t) id;
	idt_load(&idt);

	//asm("int $0x20");
}
