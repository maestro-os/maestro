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

void idt_init(void);
extern int idt_load(const void *idt);

void idt_set_state(const bool enabled);
void idt_setup_wrap(void (*handler)());

// TODO Return int?
extern void irq0();
extern void irq1();
extern void irq2();
extern void irq3();
extern void irq4();
extern void irq5();
extern void irq6();
extern void irq7();
extern void irq8();
extern void irq9();
extern void irq10();
extern void irq11();
extern void irq12();
extern void irq13();
extern void irq14();
extern void irq15();

extern void error0();
extern void error1();
extern void error2();
extern void error3();
extern void error4();
extern void error5();
extern void error6();
extern void error7();
extern void error8();
extern void error9();
extern void error10();
extern void error11();
extern void error12();
extern void error13();
extern void error14();
extern void error15();
extern void error16();
extern void error17();
extern void error18();
extern void error19();
extern void error20();
extern void error21();
extern void error22();
extern void error23();
extern void error24();
extern void error25();
extern void error26();
extern void error27();
extern void error28();
extern void error29();
extern void error30();
extern void error31();

#endif
