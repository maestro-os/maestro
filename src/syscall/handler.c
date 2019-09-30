#include <syscall/syscall.h>
#include <idt/idt.h>

// TODO remove
#include <tty/tty.h>

#define SYSCALLS_COUNT	(sizeof(sys_handlers) / sizeof(*sys_handlers))

static sys_handler_t sys_handlers[] = {
	sys_write,
	sys_fork,
	sys_exit,
	sys_getpid,
	sys_getppid,
	sys_waitpid
};

__attribute__((hot))
sys_ret_t syscall_handler(const regs_t *registers)
{
	sys_handler_t h;
	size_t id;
	process_t *process;
	sys_ret_t ret;

	id = registers->eax;
	process = get_running_process();// TODO Check if NULL?
	if(id >= SYSCALLS_COUNT || !(h = sys_handlers[id]))
	{
		process_kill(process, SIGSYS);
		kernel_loop();
	}
	process->regs_state = *registers;
	process->syscalling = true;
	STI();
	ret = h(process, registers);
	CLI();
	process->syscalling = false;
	return ret;
}
