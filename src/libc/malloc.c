#include "stdlib.h"
#include "../kernel.h"

void *malloc(size_t size)
{
	if(!size) return NULL;
	return mm_find_free(NULL, size);
}
