#include <syscall/syscall.h>

static int convert_flags(const int prot, const int flags)
{
	int f = 0;

	if(prot & PROT_WRITE)
		f |= MEM_REGION_FLAG_WRITE;
	if(prot & PROT_EXEC)
		f |= MEM_REGION_FLAG_EXEC;
	if(flags & MAP_SHARED)
		f |= MEM_REGION_FLAG_SHARED;
	return f;
}

// TODO Handle errnos
sys_ret_t sys_mmap(process_t *process, const regs_t *registers)
{
	size_t pages;
	int flags;
	int prot;
	int space_flags;
	void *ptr;

	pages = CEIL_DIVISION(registers->ebx, PAGE_SIZE);
	flags = registers->ecx;
	prot = registers->edx;
	if(!(prot & PROT_READ))
		return (sys_ret_t) NULL;
	space_flags = convert_flags(flags, prot) | MEM_REGION_FLAG_USER;
	// TODO Handle fixed map
	if(!(ptr = mem_space_alloc(process->mem_space, pages, space_flags)))
		return -errno;
	return (sys_ret_t) ptr;
}
