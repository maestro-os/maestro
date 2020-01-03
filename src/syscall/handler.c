#include <syscall/syscall.h>
#include <kernel.h>
#include <idt/idt.h>

#define SYSCALLS_COUNT	(sizeof(sys_handlers) / sizeof(*sys_handlers))

ATTR_RODATA
static const sys_handler_t sys_handlers[] = {
	sys_write,
	sys_fork,
	sys_exit,
	sys_getpid,
	sys_getppid,
	sys_waitpid
};

ATTR_HOT
sys_ret_t syscall_handler(const regs_t *registers)
{
	size_t id;
	process_t *process;
	sys_handler_t h;
	sys_ret_t ret;

	id = registers->eax;
	if(!(process = get_running_process()))
		PANIC("System call while no process is running", 0);
	if(id >= SYSCALLS_COUNT || !(h = sys_handlers[id]))
	{
		process_kill(process, SIGSYS);
		kernel_loop();
	}
	process->syscalling = 1;
	STI();
	ret = h(process, registers);
	CLI();
	process->syscalling = 0;
	return ret;
}
