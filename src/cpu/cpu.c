#include "cpu.h"

void cpu_reset(void)
{
	while(inb(0x64) & 0b10)
	{
		// TODO Sleep?
	}

	outb(0x64, 0xfe);
}
