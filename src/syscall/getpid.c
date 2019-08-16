#include <syscall/syscall.h>

sys_ret_t sys_getpid(process_t *process, const sys_info_t *info)
{
	(void) info;
	return process->pid;
}
