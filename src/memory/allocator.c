#include "memory.h"

/*static size_t alloc_size()
{
	size_t i = BLOCK_SIZE;
	while(i < (size_t) memory_end) i <<= 1;

	return i;
}

static buddy_order_t alloc_max_order(const size_t size)
{
	buddy_order_t order = 0;
	size_t i = BLOCK_SIZE;

	while(i < size)
	{
		++order;
		i <<= 1;
	}

	return order;
}

void alloc_init()
{
	void *ptr = HEAP_BEGIN;

	buddy_alloc_t *alloc = ptr;
	alloc->size = alloc_size();
	alloc->max_order = alloc_max_order(alloc->size);

	ptr += sizeof(buddy_alloc_t);

	const size_t s = ALLOC_META_SIZE(alloc->max_order);
	bzero((alloc->states = ptr), s);
	ptr += s;

	buddy_reserve_blocks(UPPER_DIVISION((uintptr_t) ptr, BLOCK_SIZE));
}*/
