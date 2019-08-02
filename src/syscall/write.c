#include <syscall/syscall.h>

// TODO temporary
#include <tty/tty.h>

sys_ret_t sys_write(const sys_info_t *info)
{
	int fildes;
	const void *buf;
	size_t nbyte;

	fildes = info->ebx;
	buf = TO_PTR(info->ecx);
	nbyte = info->edx;
	if(!buf || !vmem_contains(NULL, buf, nbyte)) // TODO Get vmem from process
	{
		// TODO Set errno
		return -1;
	}
	// TODO Write to `fildes`
	(void) fildes;
	tty_write(buf, nbyte, current_tty);
	return nbyte;
}
