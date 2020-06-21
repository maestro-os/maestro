#include <syscall/syscall.h>

sys_ret_t sys_munmap(process_t *process, const regs_t *registers)
{
	void *addr;
	size_t len;

	addr = (void *) registers->ebx;
	len = registers->ecx;
	if(!IS_ALIGNED(addr, PAGE_SIZE) || addr + len <= addr)
		return -EINVAL;
	mem_space_free(process->mem_space, addr, CEIL_DIVISION(len, PAGE_SIZE));
	return 0;
}
