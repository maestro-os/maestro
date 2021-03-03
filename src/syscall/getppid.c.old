#include <syscall/syscall.h>

sys_ret_t sys_getppid(process_t *process, const regs_t *registers)
{
	(void) registers;
	if(!process->parent)
		return 0;
	return process->parent->pid;
}
