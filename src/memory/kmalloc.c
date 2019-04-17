#include "memory.h"
#include "memory_internal.h"
#include "../libc/errno.h"

void *kmalloc(const size_t size)
{
	if(size == 0) return NULL;
	errno = 0;

	// TODO
	return NULL;
}
