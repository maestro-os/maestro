#include <syscall/syscall.h>

// TODO temporary
#include <tty/tty.h>

sys_ret_t sys_write(process_t *process, const sys_info_t *info)
{
	int fildes;
	const void *buf;
	size_t nbyte;

	fildes = info->ebx;
	buf = TO_PTR(info->ecx);
	nbyte = info->edx;
	if(!buf || !vmem_contains(get_running_process()->page_dir, buf, nbyte))
	{
		// TODO Set errno
		return -1;
	}
	// TODO Write to `fildes`
	(void) fildes;
	(void) process;
	tty_write(buf, nbyte, current_tty);
	return nbyte;
}
