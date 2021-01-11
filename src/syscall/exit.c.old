#include <syscall/syscall.h>
#include <idt/idt.h>
#include <pic/pic.h>

sys_ret_t sys_exit(process_t *process, const regs_t *registers)
{
	process_exit(process, registers->ebx);
	kernel_loop();
}
