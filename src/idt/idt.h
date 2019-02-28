#ifndef IDT_H
# define IDT_H

void idt_init();
extern int idt_load(const idt_t *idt);

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
