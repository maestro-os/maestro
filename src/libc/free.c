#include "stdlib.h"
#include "../memory/memory.h"

void free(void *ptr)
{
	if(!ptr) return;
	mm_free(ptr);
}
