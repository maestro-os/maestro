#include <syscall/syscall.h>

sys_ret_t sys_getpid(const sys_info_t *info)
{
	(void) info;
	return get_running_process()->pid;
}
