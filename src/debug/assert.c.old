#include <debug/debug.h>
#include <kernel.h>

#include <libc/stdio.h>

void _debug_assert_fail(const char *str)
{
	void *ebp;

	printf("DEBUG: Assertion failed: `%s`\n", str);
	GET_EBP(ebp);
	print_callstack(ebp, 8);
	kernel_halt();
}
