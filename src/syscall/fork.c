#include <syscall/syscall.h>

sys_ret_t sys_fork(const sys_info_t *info)
{
	process_t *proc, *child;

	(void) info;
	proc = get_running_process();
	if(!(child = process_clone(proc)))
		return -ENOMEM;
	child->tss.eax = 0;
	// TODO Increment %eip from one instruction? (on `child`)
	return child->pid;
}
