#include "memory.h"
#include "memory_internal.h"

void buddy_reserve_blocks(const size_t count)
{
	size_t i = (1 << (BIT_SIZEOF(size_t) - 1));

	while(i > 0)
	{
		if(count & i)
			// TODO
		i >>= 1;
	}
}

__attribute__((hot))
void *buddy_alloc(const size_t pages)
{
	// TODO
	(void) pages;

	return NULL;
}

__attribute__((hot))
void buddy_free(const void *ptr, const size_t pages)
{
	// TODO
	(void) ptr;
	(void) pages;
}
