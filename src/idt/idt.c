#include "../kernel.h"
#include "idt.h"
#include "../pic/pic.h"

static interrupt_descriptor_t id[48];

static interrupt_descriptor_t create_id(const void *address,
	const uint16_t selector, const uint8_t type_attr)
{
	interrupt_descriptor_t id;
	bzero(&id, sizeof(interrupt_descriptor_t));

	id.offset = ((unsigned long) address) & 0xffff;
	id.selector = selector;
	id.type_attr = type_attr;
	id.offset_2 = (((unsigned long) address) & 0xffff0000)
		>> sizeof(id.offset) * 8;

	return id;
}

void idt_init(void)
{
	pic_init(0x20, 0x28);

	// TODO Fix macros
	id[0x0] = create_id(error0, 0x8, 0x8e); // TODO Selector
	id[0x1] = create_id(error1, 0x8, 0x8e); // TODO Selector
	id[0x2] = create_id(error2, 0x8, 0x8e); // TODO Selector
	id[0x3] = create_id(error3, 0x8, 0x8e); // TODO Selector
	id[0x4] = create_id(error4, 0x8, 0x8e); // TODO Selector
	id[0x5] = create_id(error5, 0x8, 0x8e); // TODO Selector
	id[0x6] = create_id(error6, 0x8, 0x8e); // TODO Selector
	id[0x7] = create_id(error7, 0x8, 0x8e); // TODO Selector
	id[0x8] = create_id(error8, 0x8, 0x8e); // TODO Selector
	id[0x9] = create_id(error9, 0x8, 0x8e); // TODO Selector
	id[0xa] = create_id(error10, 0x8, 0x8e); // TODO Selector
	id[0xb] = create_id(error11, 0x8, 0x8e); // TODO Selector
	id[0xc] = create_id(error12, 0x8, 0x8e); // TODO Selector
	id[0xd] = create_id(error13, 0x8, 0x8e); // TODO Selector
	id[0xe] = create_id(error14, 0x8, 0x8e); // TODO Selector
	id[0xf] = create_id(error15, 0x8, 0x8e); // TODO Selector
	id[0x10] = create_id(error16, 0x8, 0x8e); // TODO Selector
	id[0x11] = create_id(error17, 0x8, 0x8e); // TODO Selector
	id[0x12] = create_id(error18, 0x8, 0x8e); // TODO Selector
	id[0x13] = create_id(error19, 0x8, 0x8e); // TODO Selector
	id[0x14] = create_id(error20, 0x8, 0x8e); // TODO Selector
	id[0x15] = create_id(error21, 0x8, 0x8e); // TODO Selector
	id[0x16] = create_id(error22, 0x8, 0x8e); // TODO Selector
	id[0x17] = create_id(error23, 0x8, 0x8e); // TODO Selector
	id[0x18] = create_id(error24, 0x8, 0x8e); // TODO Selector
	id[0x19] = create_id(error25, 0x8, 0x8e); // TODO Selector
	id[0x1a] = create_id(error26, 0x8, 0x8e); // TODO Selector
	id[0x1b] = create_id(error27, 0x8, 0x8e); // TODO Selector
	id[0x1c] = create_id(error28, 0x8, 0x8e); // TODO Selector
	id[0x1d] = create_id(error29, 0x8, 0x8e); // TODO Selector
	id[0x1e] = create_id(error30, 0x8, 0x8e); // TODO Selector
	id[0x1f] = create_id(error31, 0x8, 0x8e); // TODO Selector

	id[0x20] = create_id(irq0, 0x8, 0x8e); // TODO Selector
	id[0x21] = create_id(irq1, 0x8, 0x8e); // TODO Selector
	id[0x22] = create_id(irq2, 0x8, 0x8e); // TODO Selector
	id[0x23] = create_id(irq3, 0x8, 0x8e); // TODO Selector
	id[0x24] = create_id(irq4, 0x8, 0x8e); // TODO Selector
	id[0x25] = create_id(irq5, 0x8, 0x8e); // TODO Selector
	id[0x26] = create_id(irq6, 0x8, 0x8e); // TODO Selector
	id[0x27] = create_id(irq7, 0x8, 0x8e); // TODO Selector
	id[0x28] = create_id(irq8, 0x8, 0x8e); // TODO Selector
	id[0x29] = create_id(irq9, 0x8, 0x8e); // TODO Selector
	id[0x2a] = create_id(irq10, 0x8, 0x8e); // TODO Selector
	id[0x2b] = create_id(irq11, 0x8, 0x8e); // TODO Selector
	id[0x2c] = create_id(irq12, 0x8, 0x8e); // TODO Selector
	id[0x2d] = create_id(irq13, 0x8, 0x8e); // TODO Selector
	id[0x2e] = create_id(irq14, 0x8, 0x8e); // TODO Selector
	id[0x2f] = create_id(irq15, 0x8, 0x8e); // TODO Selector

	unsigned long idt_ptr[2];
	idt_ptr[0] = sizeof(id) + (((unsigned long) id & 0xffff) << 16);
	idt_ptr[1] = ((unsigned long) id) >> 16;
	idt_load(idt_ptr);

	idt_set_state(true);
}

void idt_set_state(const bool enabled)
{
	if(enabled)
		asm("sti");
	else
		asm("cli");
}

void idt_setup_wrap(void (*handler)())
{
	idt_set_state(false);
	handler();
	idt_set_state(true);
}
