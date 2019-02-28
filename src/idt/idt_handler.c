#include "../pic/pic.h"
#include "../libc/stdio.h"

void irq0_handler()
{
	printf("A");
	// TODO
	pic_EOI(0x0);
}

void irq1_handler()
{
	printf("KEYBOARD");
	// TODO
	pic_EOI(0x1);
}

void irq2_handler()
{
	printf("A");
	// TODO
	pic_EOI(0x2);
}

void irq3_handler()
{
	printf("A");
	// TODO
	pic_EOI(0x3);
}

void irq4_handler()
{
	printf("A");
	// TODO
	pic_EOI(0x4);
}

void irq5_handler()
{
	printf("A");
	// TODO
	pic_EOI(0x5);
}

void irq6_handler()
{
	printf("A");
	// TODO
	pic_EOI(0x6);
}

void irq7_handler()
{
	printf("A");
	// TODO
	pic_EOI(0x7);
}

void irq8_handler()
{
	printf("A");
	// TODO
	pic_EOI(0x8);
}

void irq9_handler()
{
	printf("A");
	// TODO
	pic_EOI(0x9);
}

void irq10_handler()
{
	printf("A");
	// TODO
	pic_EOI(0xa);
}

void irq11_handler()
{
	printf("A");
	// TODO
	pic_EOI(0xb);
}

void irq12_handler()
{
	printf("A");
	// TODO
	pic_EOI(0xc);
}

void irq13_handler()
{
	printf("A");
	// TODO
	pic_EOI(0xd);
}

void irq14_handler()
{
	printf("A");
	// TODO
	pic_EOI(0xe);
}

void irq15_handler()
{
	printf("A");
	// TODO
	pic_EOI(0xf);
}
