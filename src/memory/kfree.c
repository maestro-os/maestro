#include <memory/memory.h>

void kfree(void *ptr)
{
	if(!ptr)
		return;
	// TODO Get cache for ptr
	// TODO Free from cache
}
