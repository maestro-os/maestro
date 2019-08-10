#include <memory/kmalloc/kmalloc.h>

void *krealloc(void *ptr, const size_t size, const int flags)
{
	if(!ptr)
		return kmalloc(size, flags);
	if(size == 0)
	{
		kfree(ptr, flags);
		return NULL;
	}
	// TODO
	return NULL;
}
