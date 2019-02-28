#include "../kernel.h"

extern int irq0();
extern int irq1();
extern int irq2();
extern int irq3();
extern int irq4();
extern int irq5();
extern int irq6();
extern int irq7();
extern int irq8();
extern int irq9();
extern int irq10();
extern int irq11();
extern int irq12();
extern int irq13();
extern int irq14();
extern int irq15();

static interrupt_descriptor_t create_id(const void *offset,
	const uint16_t selector, const uint8_t type_attr)
{
	interrupt_descriptor_t id;
	id.offset = ((uint32_t) offset) & 0xffff;
	id.selector = selector;
	id.zero = 0;
	id.type_attr = type_attr;
	id.offset_2 = (((uint32_t) offset) & 0xffff0000) >> sizeof(id.offset) * 2;

	return id;
}

void idt_init()
{
	// TODO PIC remapping

	interrupt_descriptor_t id[48];
	id[32] = create_id(irq0, 0x8, ID_TYPE_GATE_INTERRUPT32
		| ID_PRIVILEGE_RING_0 | ID_PRESENT); // TODO Selector
	id[33] = create_id(irq1, 0x8, ID_TYPE_GATE_INTERRUPT32
		| ID_PRIVILEGE_RING_0 | ID_PRESENT); // TODO Selector
	id[34] = create_id(irq2, 0x8, ID_TYPE_GATE_INTERRUPT32
		| ID_PRIVILEGE_RING_0 | ID_PRESENT); // TODO Selector
	id[35] = create_id(irq3, 0x8, ID_TYPE_GATE_INTERRUPT32
		| ID_PRIVILEGE_RING_0 | ID_PRESENT); // TODO Selector
	id[36] = create_id(irq4, 0x8, ID_TYPE_GATE_INTERRUPT32
		| ID_PRIVILEGE_RING_0 | ID_PRESENT); // TODO Selector
	id[37] = create_id(irq5, 0x8, ID_TYPE_GATE_INTERRUPT32
		| ID_PRIVILEGE_RING_0 | ID_PRESENT); // TODO Selector
	id[38] = create_id(irq6, 0x8, ID_TYPE_GATE_INTERRUPT32
		| ID_PRIVILEGE_RING_0 | ID_PRESENT); // TODO Selector
	id[39] = create_id(irq7, 0x8, ID_TYPE_GATE_INTERRUPT32
		| ID_PRIVILEGE_RING_0 | ID_PRESENT); // TODO Selector
	id[40] = create_id(irq8, 0x8, ID_TYPE_GATE_INTERRUPT32
		| ID_PRIVILEGE_RING_0 | ID_PRESENT); // TODO Selector
	id[41] = create_id(irq9, 0x8, ID_TYPE_GATE_INTERRUPT32
		| ID_PRIVILEGE_RING_0 | ID_PRESENT); // TODO Selector
	id[42] = create_id(irq10, 0x8, ID_TYPE_GATE_INTERRUPT32
		| ID_PRIVILEGE_RING_0 | ID_PRESENT); // TODO Selector
	id[43] = create_id(irq11, 0x8, ID_TYPE_GATE_INTERRUPT32
		| ID_PRIVILEGE_RING_0 | ID_PRESENT); // TODO Selector
	id[44] = create_id(irq12, 0x8, ID_TYPE_GATE_INTERRUPT32
		| ID_PRIVILEGE_RING_0 | ID_PRESENT); // TODO Selector
	id[45] = create_id(irq13, 0x8, ID_TYPE_GATE_INTERRUPT32
		| ID_PRIVILEGE_RING_0 | ID_PRESENT); // TODO Selector
	id[46] = create_id(irq14, 0x8, ID_TYPE_GATE_INTERRUPT32
		| ID_PRIVILEGE_RING_0 | ID_PRESENT); // TODO Selector
	id[47] = create_id(irq15, 0x8, ID_TYPE_GATE_INTERRUPT32
		| ID_PRIVILEGE_RING_0 | ID_PRESENT); // TODO Selector

	idt_t idt;
	idt.limit = sizeof(interrupt_descriptor_t) * 48; // TODO
	idt.base = (uint32_t) id;

	idt_load(&idt);
}
