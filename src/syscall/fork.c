#include <syscall/syscall.h>

sys_ret_t sys_fork(process_t *process, const regs_t *registers)
{
	process_t *child;

	(void) registers;
	if(!(child = process_clone(process)))
		return -ENOMEM;
	child->regs_state = *registers;
	child->regs_state.eax = 0;
	return child->pid;
}
