#include <memory/memory.h>

void *krealloc(void *ptr, const size_t size)
{
	if(!ptr)
		return kmalloc(size);
	if(size == 0)
	{
		kfree(ptr);
		return NULL;
	}
	// TODO
	return NULL;
}
