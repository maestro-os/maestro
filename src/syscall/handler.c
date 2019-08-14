#include <syscall/syscall.h>

// TODO temporary
#include <tty/tty.h>

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

	id = info->eax;
	if(id >= SYSCALLS_COUNT || !(h = sys_handlers[id]))
	{
		// TODO Bad syscall. Kill process?
		return -1;
	}
	return h(info);
}
