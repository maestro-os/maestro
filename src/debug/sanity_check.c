#include <debug/debug.h>
#include <kernel.h>
#include <memory/memory.h>

#include <libc/stdio.h>

int _debug_sanity_check(const void *ptr)
{
	void *ebp;

	if(ptr && (ptr < KERNEL_BEGIN || ptr >= mem_info.memory_end))
	{
		printf("DEBUG: Sanity check failed: `%p`\n", ptr);
		GET_EBP(ebp);
		print_callstack(ebp, 8);
		kernel_halt();
	}
	return (ptr != NULL);
}
