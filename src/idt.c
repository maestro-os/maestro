#include "kernel.h"

extern int load_idt();

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

void load_idt(idt_t *idt)
{
	interrupt_descriptor_t *id = (void *) idt->base;
	// TODO

	idt->limit = 0; // TODO
}
