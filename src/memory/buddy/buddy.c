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
	size_t index = BUDDY_INDEX(max_order, order, i);

	// TODO Set/clear parent? (clear if both children are free)

	if (used)
		bitmap_set(states, index);
	else
		bitmap_clear(states, index);

	if(order > 0)
	{
		buddy_set_block(i * 2, order - 1, used);
		buddy_set_block(i * 2 + 1, order - 1, used);
	}
}

static size_t find_buddy(const size_t order)
{
	for(size_t i = 0; i < BLOCKS_COUNT(max_order, order); ++i)
	{
		if(!buddy_get_block(i, order))
			return i;
	}

	return BUDDY_NULL;
}

static size_t split_block(const buddy_order_t order, const size_t i,
	const size_t n)
{
	if(order == 0 || n == 0) return BUDDY_NULL;

	buddy_set_block(order, i, 1);
	return split_block(order - 1, i * 2, n - 1);
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
	return BUDDY_PTR(order - n, split_block(order + n, buddy, n));
}

__attribute__((hot))
void buddy_free(void *ptr)
{
	// TODO
	(void) ptr;
}
