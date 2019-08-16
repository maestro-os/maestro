#include <syscall/syscall.h>

sys_ret_t sys_fork(process_t *process, const sys_info_t *info)
{
	process_t *child;

	(void) info;
	if(!(child = process_clone(process)))
		return -ENOMEM;
	child->tss.eax = 0;
	// TODO Increment %eip from one instruction? (on `child`)
	return child->pid;
}
