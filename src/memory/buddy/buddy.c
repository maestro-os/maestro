#include <memory/buddy/buddy.h>
#include <idt/idt.h>
#include <libc/errno.h>

// TODO Fix: Infinite loop if memory is full

static block_order_t max_order;
static block_state_t *states;
static void *buddy_begin;
static size_t end;

static spinlock_t spinlock = 0;

ATTR_HOT
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

ATTR_HOT
void *buddy_get_begin(void)
{
	return buddy_begin;
}

ATTR_HOT
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

ATTR_HOT
static inline void set_block_state(const block_index_t index,
	const block_state_t state)
{
	states[index] = state;
	if(index > 0)
		update_block_state(NODE_PARENT(index));
}

ATTR_COLD
void buddy_init(void)
{
	size_t metadata_size;
	void *buddy_end;
	size_t end_end;
	size_t i;

	max_order = buddy_get_order(mem_info.available_memory);
	states = mem_info.heap_begin;
	metadata_size = METADATA_SIZE(max_order);
	bzero(states, metadata_size);
	buddy_begin = ALIGN_UP(states + metadata_size, PAGE_SIZE);

	buddy_end = ALIGN_DOWN(mem_info.heap_end, PAGE_SIZE);
	end = NODES_COUNT(max_order - 1)
		+ ((uintptr_t) (buddy_end - buddy_begin) / PAGE_SIZE);
	end_end = NODES_COUNT(max_order);
	for(i = end; i < end_end; ++i)
		set_block_state(i, NODE_STATE_FULL);
}

ATTR_HOT
static block_index_t find_free(const block_index_t index,
	const block_order_t order, const int is_buddy)
{
	block_order_t block_order;
	block_state_t block_state;
	block_index_t i;

	if(order >= max_order)
		return -1;
	block_order = NODE_ORDER(max_order, index);
	if(block_order < order)
		return -1;
	block_state = states[index];
	if(block_order == 0 && block_state == NODE_STATE_FULL)
		return -1;
	switch(block_state)
	{
		case NODE_STATE_FREE:
		{
			if(block_order > order)
				return find_free(NODE_LEFT(index), order, 0);
			else if(block_order == order)
				return index;
			break;
		}

		case NODE_STATE_PARTIAL:
		{
			if(block_order <= order)
				break;
			if((i = find_free(NODE_LEFT(index), order, 0)) >= 0)
				return i;
		}

		case NODE_STATE_FULL: break;
	}
	if(index > 0 && !is_buddy)
		return find_free(NODE_BUDDY(index), order, 1);
	return -1;
}

ATTR_HOT
ATTR_MALLOC
void *buddy_alloc(const block_order_t order)
{
	block_index_t block;
	void *ptr;

	spin_lock(&spinlock);
	errno = 0;
	if((block = find_free(0, order, 0)) >= 0)
	{
		set_block_state(block, NODE_STATE_FULL);
		ptr = NODE_PTR(buddy_begin, max_order, block);
	}
	else
	{
		errno = ENOMEM;
		ptr = NULL;
	}
	spin_unlock(&spinlock);
	return ptr;
}

ATTR_HOT
ATTR_MALLOC
void *buddy_alloc_zero(const block_order_t order)
{
	void *ptr;

	if((ptr = buddy_alloc(order)))
		bzero(ptr, BLOCK_SIZE(order));
	return ptr;
}

ATTR_HOT
void buddy_free(void *ptr)
{
	block_index_t index;
	block_order_t order = 0;

	spin_lock(&spinlock);
	index = NODES_COUNT(max_order - 1)
		+ ((uintptr_t) (ptr - buddy_begin) / PAGE_SIZE);
	while(order < max_order && states[index] != NODE_STATE_FULL)
	{
		index = NODE_PARENT(index);
		++order;
	}
	set_block_state(index, NODE_STATE_FREE);
	spin_unlock(&spinlock);
}

ATTR_HOT
static size_t count_allocated_pages(const size_t index)
{
	size_t order;

	if(index >= end)
		return 0;
	order = NODE_ORDER(max_order, index);
	if(order == 0 && states[index] == NODE_STATE_FULL)
		return 1;
	return count_allocated_pages(NODE_LEFT(index))
		+ count_allocated_pages(NODE_RIGHT(index));
}

ATTR_HOT
inline size_t allocated_pages(void)
{
	return count_allocated_pages(0);
}
