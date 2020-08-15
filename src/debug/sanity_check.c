#include <debug/debug.h>
#include <kernel.h>
#include <memory/memory.h>

#include <libc/stdio.h>

void *_debug_sanity_check(const volatile void *ptr)
{
	void *ebp;

	if(ptr && (ptr < KERNEL_VIRT_BEGIN || ptr >= mem_info.memory_end))
	{
		printf("DEBUG: Sanity check failed: `%p`\n", ptr);
		GET_EBP(ebp);
		print_callstack(ebp, 8);
		kernel_halt();
	}
	return (void *) ptr;
}
