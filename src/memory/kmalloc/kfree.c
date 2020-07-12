#include <memory/kmalloc/kmalloc.h>
#include <debug/debug.h>

void kfree(void *ptr)
{
	if(!sanity_check(ptr))
		return;
	// TODO
}
