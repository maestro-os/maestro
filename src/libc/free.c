#include <libc/stdlib.h>
#include <memory/memory.h>

void free(void *ptr)
{
	if(!ptr) return;
	// TODO mm_free(ptr);
}
