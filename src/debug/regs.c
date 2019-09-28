#include <debug/debug.h>

void print_regs(const regs_t *regs)
{
	if(!regs)
		return;
	printf("--- Registers ---\n");
	printf("ebp: %p\n", (void *) regs->ebp);
	printf("esp: %p\n", (void *) regs->esp);
	printf("eip: %p\n", (void *) regs->eip);
	printf("eflags: %i\n", (int) regs->eflags);
	printf("eax: %x\n", (int) regs->eax);
	printf("ebx: %x\n", (int) regs->ebx);
	printf("ecx: %x\n", (int) regs->ecx);
	printf("edx: %x\n", (int) regs->edx);
	printf("esi: %x\n", (int) regs->esi);
	printf("edi: %x\n", (int) regs->edi);
}
