#include "buddy.h"

static size_t buddy_size;
static buddy_order_t max_order;
static char *states;

// TODO Free list

__attribute__((cold))
static size_t get_buddy_size()
{
	size_t i = PAGE_SIZE;
	while(i < (size_t) memory_end) i <<= 1;

	return i;
}

__attribute__((hot))
buddy_order_t buddy_get_order(const size_t size)
{
	buddy_order_t order = 0;
	size_t i = PAGE_SIZE;

	while(i < size)
	{
		++order;
		i <<= 1;
	}

	return order;
}

__attribute__((cold))
void buddy_init()
{
	max_order = buddy_get_order((buddy_size = get_buddy_size()));
	states = HEAP_BEGIN;

	size_t buddy_states_size = BUDDY_STATES_SIZE(buddy_size);
	bzero(states, buddy_states_size);

	void *available_begin = ALIGN(states + buddy_states_size, PAGE_SIZE);
	buddy_set_block(0, buddy_get_order((size_t) available_begin), 1);

	void *available_end = ALIGN_DOWN(memory_end, PAGE_SIZE);
	size_t end_order = buddy_get_order(buddy_size - (size_t) available_end);
	size_t end_index = ((uintptr_t) available_end / PAGE_SIZE)
		/ POW2(end_order);
	buddy_set_block(end_index, end_order, 1);

	// TODO Alloc free list
}

int buddy_get_block(const size_t i, const buddy_order_t order)
{
	if(order > max_order) return 1;
	return bitmap_get(states, BUDDY_INDEX(max_order, order, i));
}

void buddy_set_block(const size_t i, const buddy_order_t order, const int used)
{
	if(order > max_order) return;
	const size_t index = BUDDY_INDEX(max_order, order, i);

	if(used)
		bitmap_set(states, index);
	else
		bitmap_clear(states, index);
}

__attribute__((hot))
static size_t find_buddy(const buddy_order_t order)
{
	for(size_t i = 0; i < PAGES_COUNT(max_order, order); ++i)
	{
		if(!buddy_get_block(i, order))
			return i;
	}

	return BUDDY_NULL;
}

__attribute__((hot))
static size_t split_block(const buddy_order_t order, const size_t i,
	const size_t n)
{
	buddy_set_block(i, order, 1);
	return (n == 0 ? i : split_block(order - 1, i * 2, n - 1));
}

__attribute__((hot))
void *buddy_alloc(const size_t order)
{
	// TODO Check free list
	// TODO If no block large enough is in free list, look for a block in buddies

	size_t buddy;
	buddy_order_t n = 0;

	do
		buddy = find_buddy(order + n);
	while((buddy == BUDDY_NULL) && (order + ++n < max_order));

	if(buddy == BUDDY_NULL) return NULL;
	if(n == 0) return BUDDY_PTR(order, buddy);
	return BUDDY_PTR(order, split_block(order + n, buddy, n));
}

void *buddy_alloc_zero(const size_t order)
{
	void *ptr = buddy_alloc(order);
	bzero(ptr, BLOCK_SIZE(max_order, order));
	return ptr;
}

__attribute__((hot))
void buddy_free(void *ptr)
{
	// TODO
	(void) ptr;
}
