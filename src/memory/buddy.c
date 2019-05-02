#include "memory.h"

static size_t buddy_size, max_order;
static char *states;

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
	max_order = get_order((buddy_size = get_buddy_size(size)));
	states = HEAP_BEGIN;

	const size_t buddy_states_size = BUDDY_STATES_SIZE(size);
	bzero(states, buddy_states_size);

	void *available_begin = ALIGN(states + buddy_states_size, PAGE_SIZE);
	buddy_set_block(0, get_order((size_t) available_begin), 1);

	void *available_end = ALIGN_DOWN(memory_end, PAGE_SIZE);
	const size_t end_index = (available_end / PAGE_SIZE) / POW2(end_order);
	const size_t end_order = get_order((size_t) buddy_size - available_end);
	buddy_set_block(end_index, end_order, 1);
}

void buddy_set_block(const size_t i, const size_t order, const int used)
{
	// TODO Get block and set state
	(void) i;
	(void) order;
	(void) used;
}

__attribute__((hot))
void *buddy_alloc(const size_t pages)
{
	// TODO
	(void) pages;

	return NULL;
}

__attribute__((hot))
void buddy_free(void *ptr)
{
	// TODO
	(void) ptr;
}
