#include <cpu/cpu.h>

/*
 * Triggers a CPU reset.
 */
void cpu_reset(void)
{
	while(inb(0x64) & 0b10)
		;
	outb(0x64, 0xfe);
}
