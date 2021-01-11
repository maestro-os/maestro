#include <debug/debug.h>
#include <process/process.h>

#include <libc/stdio.h>

void print_regs(const regs_t *regs)
{
	if(!regs)
		return;
	printf("ebp: %#.8x ", (int) regs->ebp);
	printf("esp: %#.8x ", (int) regs->esp);
	printf("eip: %#.8x ", (int) regs->eip);
	printf("eflags: %#.8x ", (int) regs->eflags);
	printf("eax: %#.8x\n", (int) regs->eax);
	printf("ebx: %#.8x ", (int) regs->ebx);
	printf("ecx: %#.8x ", (int) regs->ecx);
	printf("edx: %#.8x ", (int) regs->edx);
	printf("esi: %#.8x ", (int) regs->esi);
	printf("edi: %#.8x\n", (int) regs->edi);
}
