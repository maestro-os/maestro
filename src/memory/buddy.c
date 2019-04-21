#include "memory.h"
#include "memory_internal.h"

__attribute__((hot))
static inline void buddy_toggle(buddy_alloc_t *alloc,
	const buddy_order_t order, const size_t i)
{
	bitmap_toggle(alloc->states, BUDDY_INDEX(alloc->max_order, order, i));
}

void buddy_reserve_blocks(const size_t count)
{
	size_t blocks = (1 << (BIT_SIZEOF(size_t) - 1));
	size_t j = 0;

	while(blocks > 0)
	{
		if(count & blocks)
		{
			// TODO While buddy is larger than `blocks`, toggle it and go to child
			// TODO Toggle buddies from `j` to `j + blocks` at order `0`
			j |= blocks;
		}

		blocks >>= 1;
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
