#include <cpu/cpu.h>

void cpu_reset(void)
{
	// TODO Enable CPUID in EFLAGS

	while(inb(0x64) & 0b10)
	{
		// TODO Sleep?
	}

	outb(0x64, 0xfe);
}
