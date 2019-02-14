#include "stdlib.h"

void *realloc(void *ptr, size_t size)
{
	if(!ptr) return malloc(size);

	if(!size)
	{
		free((void *) ptr);
		return NULL;
	}

	// TODO

	return NULL;
}
