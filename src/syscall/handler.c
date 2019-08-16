#include <syscall/syscall.h>
#include <idt/idt.h>

#define SYSCALLS_COUNT	(sizeof(sys_handlers) / sizeof(*sys_handlers))

static sys_handler_t sys_handlers[] = {
	sys_write,
	sys_fork,
	sys_exit,
	sys_getpid,
	sys_waitpid
};

__attribute__((hot))
sys_ret_t syscall_handler(const sys_info_t *info)
{
	sys_handler_t h;
	size_t id;
	process_t *process;
	sys_ret_t ret;

	id = info->eax;
	if(id >= SYSCALLS_COUNT || !(h = sys_handlers[id]))
	{
		// TODO Bad syscall. Kill process?
		return -1;
	}
	process = get_running_process();// TODO Check if NULL?
	process->syscalling = true;
	STI();
	ret = h(process, info);
	process->syscalling = false;
	return ret;
}
