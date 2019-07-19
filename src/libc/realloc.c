#include <libc/stdlib.h>

void *realloc(void *ptr, size_t size)
{
	if(!ptr) return malloc(size);

	if(!size)
	{
		free((void *) ptr);
		return NULL;
	}

	// TODO
	free(ptr);

	if(!(ptr = malloc(size)))
		return NULL;

	// TODO Copy previous data
	return ptr;
}
