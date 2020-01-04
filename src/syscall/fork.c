#include <syscall/syscall.h>
#include <idt/idt.h>

sys_ret_t sys_fork(process_t *process, const regs_t *registers)
{
	process_t *child;

	CLI();
	if(!(child = process_clone(process)))
		return -ENOMEM;
	child->regs_state = *registers;
	child->regs_state.eax = 0;
	STI();
	return child->pid;
}
