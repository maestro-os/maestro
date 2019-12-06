#include <syscall/syscall.h>

// TODO temporary
#include <tty/tty.h>

// TODO tmp
semaphore_t sem;

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
	sem_wait(&sem, process);
	CLI(); // TODO rm
	// TODO Write to `fildes`
	(void) fildes;
	tty_write(buf, nbyte, current_tty);
	STI(); // TODO rm
	sem_post(&sem);
	return nbyte;
}
