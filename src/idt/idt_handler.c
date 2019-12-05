#include <pic/pic.h>
#include <keyboard/ps2/ps2.h>
#include <rtc/rtc.h>
#include <disk/ata/ata.h>

void irq0_handler()
{
	time_update();
	pic_EOI(0x0);
}

void irq1_handler()
{
	ps2_keyboard_event();
	pic_EOI(0x1);
}

void irq2_handler()
{
	// TODO
	pic_EOI(0x2);
}

void irq3_handler()
{
	// TODO
	pic_EOI(0x3);
}

void irq4_handler()
{
	// TODO
	pic_EOI(0x4);
}

void irq5_handler()
{
	// TODO
	pic_EOI(0x5);
}

void irq6_handler()
{
	// TODO
	pic_EOI(0x6);
}

void irq7_handler()
{
	// TODO
	pic_EOI(0x7);
}

void irq9_handler()
{
	// TODO
	pic_EOI(0x9);
}

void irq10_handler()
{
	// TODO
	pic_EOI(0xa);
}

void irq11_handler()
{
	// TODO
	pic_EOI(0xb);
}

void irq12_handler()
{
	// TODO
	pic_EOI(0xc);
}

void irq13_handler()
{
	// TODO
	pic_EOI(0xd);
}

void irq14_handler()
{
	ata_irq();
	pic_EOI(0xe);
}

void irq15_handler()
{
	// TODO
	pic_EOI(0xf);
}
