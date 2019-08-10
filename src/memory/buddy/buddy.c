#include <memory/buddy/buddy.h>
#include <idt/idt.h>
#include <libc/errno.h>

// TODO Set errnos

static block_order_t max_order;
static block_state_t *states;

// TODO Free list

static spinlock_t spinlock = 0;

__attribute__((hot))
block_order_t buddy_get_order(const size_t size)
{
	block_order_t order = 0;
	size_t i = 1;

	while(i < size / PAGE_SIZE)
	{
		i <<= 1;
		++order;
	}
	return order;
}

__attribute__((hot))
static void update_block_state(size_t index)
{
	block_state_t left_state, right_state;

	while(1)
	{
		left_state = states[NODE_LEFT(index)];
		right_state = states[NODE_RIGHT(index)];
		if((left_state | right_state) == NODE_STATE_FREE)
			states[index] = NODE_STATE_FREE;
		else if((left_state & right_state) == NODE_STATE_FULL)
			states[index] = NODE_STATE_FULL;
		else
			states[index] = NODE_STATE_PARTIAL;
		if(index == 0)
			break;
		index = NODE_PARENT(index);
	}
}

__attribute__((hot))
static inline void set_block_state(const size_t index,
	const block_state_t state)
{
	states[index] = state;
	if(index > 0)
		update_block_state(NODE_PARENT(index));
}

__attribute__((cold))
void buddy_init(void)
{
	size_t metadata_size;
	void *buddy_end;
	size_t end_begin, end_end;
	size_t i;

	max_order = buddy_get_order(available_memory);
	states = heap_begin;
	metadata_size = METADATA_SIZE(max_order);
	bzero(states, metadata_size);
	buddy_begin = ALIGN_UP(states + metadata_size, PAGE_SIZE);

	buddy_end = ALIGN_DOWN(heap_end, PAGE_SIZE);
	end_begin = NODES_COUNT(max_order - 1)
		+ ((uintptr_t) (buddy_end - buddy_begin) / PAGE_SIZE);
	end_end = NODES_COUNT(max_order);
	for(i = end_begin; i < end_end; ++i)
		set_block_state(i, NODE_STATE_FULL);

	// TODO Free list
}

__attribute__((hot))
static size_t find_free(const size_t index, const block_order_t order,
	const bool is_buddy)
{
	block_order_t block_order;
	block_state_t block_state;
	size_t i;

	if(order >= max_order)
		return BLOCK_NULL;
	block_order = NODE_ORDER(max_order, index);
	if(block_order < order)
		return BLOCK_NULL;
	block_state = states[index];
	if(block_order == 0 && block_state == NODE_STATE_FULL)
		return BLOCK_NULL;
	switch(block_state)
	{
		case NODE_STATE_FREE:
		{
			if(block_order > order)
				return find_free(NODE_LEFT(index), order, false);
			else if(block_order == order)
				return index;
			break;
		}

		case NODE_STATE_PARTIAL:
		{
			if(block_order <= order)
				break;
			if((i = find_free(NODE_LEFT(index), order, false)) != BLOCK_NULL)
				return i;
		}

		case NODE_STATE_FULL: break;
	}
	if(index > 0 && !is_buddy)
		return find_free(NODE_BUDDY(index), order, true);
	return BLOCK_NULL;
}

__attribute__((hot))
void *buddy_alloc(const block_order_t order)
{
	size_t block;
	void *ptr;

	lock(&spinlock);
	// TODO Check free list
	block = find_free(0, order, false);
	if(block != BLOCK_NULL)
	{
		set_block_state(block, NODE_STATE_FULL);
		ptr = NODE_PTR(buddy_begin, max_order, block);
	}
	else
	{
		errno = ENOMEM;
		ptr = NULL;
	}
	unlock(&spinlock);
	return ptr;
}

void *buddy_alloc_zero(const block_order_t order)
{
	void *ptr;

	if((ptr = buddy_alloc(order)))
		bzero(ptr, BLOCK_SIZE(order));
	return ptr;
}

__attribute__((hot))
void buddy_free(void *ptr)
{
	size_t index;
	size_t order = 0;

	lock(&spinlock);
	index = NODES_COUNT(max_order - 1)
		+ ((uintptr_t) (ptr - buddy_begin) / PAGE_SIZE);
	while(order < max_order && states[index] != NODE_STATE_FULL)
	{
		index = NODE_PARENT(index);
		++order;
	}
	set_block_state(index, NODE_STATE_FREE);
	// TODO Add to free list if necessary
	unlock(&spinlock);
}

__attribute__((hot))
static size_t count_allocated_pages(const size_t index)
{
	// TODO Count every allocated page, excluding unusable ones
	(void) index;
	return 0;
}

__attribute__((hot))
inline size_t allocated_pages(void)
{
	return count_allocated_pages(0);
}
