#include <syscall/syscall.h>

sys_ret_t sys_getpid(process_t *process, const regs_t *registers)
{
	(void) registers;
	return process->pid;
}
