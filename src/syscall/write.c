#include <syscall/syscall.h>

// TODO temporary
#include <tty/tty.h>

sys_ret_t sys_write(process_t *process, const regs_t *registers)
{
	int fildes;
	const void *buf;
	size_t nbyte;

	fildes = registers->ebx;
	buf = (void *) registers->ecx;
	nbyte = registers->edx;
	if(!buf || !vmem_contains(process->page_dir, buf, nbyte))
	{
		// TODO Set errno
		return -1;
	}
	// TODO Write to `fildes`
	(void) fildes;
	tty_write(buf, nbyte, current_tty);
	return nbyte;
}
