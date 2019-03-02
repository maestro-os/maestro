#ifndef IDT_H
# define IDT_H

# define ID_TYPE_GATE_TASK			0b01010000
# define ID_TYPE_GATE_INTERRUPT16	0b01100000
# define ID_TYPE_GATE_TRAP16		0b01110000
# define ID_TYPE_GATE_INTERRUPT32	0b11100000
# define ID_TYPE_GATE_TRAP32		0b11110000
# define ID_TYPE_S					0b00001000
# define ID_PRIVILEGE_RING_0		0b00000000
# define ID_PRIVILEGE_RING_1		0b00000010
# define ID_PRIVILEGE_RING_2		0b00000100
# define ID_PRIVILEGE_RING_3		0b00000110
# define ID_PRESENT					0b00000001

typedef struct interrupt_descriptor
{
	uint16_t offset;
	uint16_t selector;
	uint8_t zero;
	uint8_t type_attr;
	uint16_t offset_2;
} interrupt_descriptor_t;

void idt_init();
extern int idt_load(const void *idt);

void set_interrupts_state(const int enabled);

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

void end_of_interrupt(const uint8_t irq);

#endif
