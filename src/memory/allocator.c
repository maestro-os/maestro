#include "memory.h"

static size_t largest_alloc()
{
	size_t i = BLOCK_SIZE;
	while((i << 1) < (size_t) memory_end) i <<= 1;

	return i;
}

static size_t get_buddies_count()
{
	size_t covered = 0;
	size_t i = largest_alloc();
	size_t buddies_count = 0;

	while((size_t) memory_end - covered > BUDDY_MIN_SIZE)
	{
		covered += i;
		i >>= 2;
		++buddies_count;
	}

	return buddies_count;
}

static buddy_order_t alloc_max_order(const size_t size)
{
	buddy_order_t order = 0;
	size_t i = PAGE_SIZE;

	while(i < size)
	{
		++order;
		i *= 2;
	}

	return order;
}

void buddy_init()
{
	void *ptr = HEAP_BEGIN;

	const size_t largest = largest_alloc();
	const size_t buddies_count = get_buddies_count();

	bzero(ptr, buddies_count * sizeof(buddy_alloc_t));

	buddy_alloc_t *prev = NULL;

	for(size_t i = 0; i < buddies_count; ++i)
	{
		buddy_alloc_t *a = ptr;

		if(prev)
		{
			a->begin = prev->begin + prev->size;
			a->size = prev->size << 1;
		}
		else
			a->size = largest;
		a->max_order = alloc_max_order(a->size);

		ptr += sizeof(buddy_alloc_t);

		const size_t s = ALLOC_META_SIZE(alloc_max_order(a->size));
		bzero((a->states = ptr), s);
		ptr += s;

		a->next = (i + 1 < buddies_count ? ptr : NULL);
		prev = a;

		if(!prev) allocators = a;
	}

	buddy_reserve_blocks(UPPER_DIVISION((uintptr_t) ptr, BLOCK_SIZE));
}
