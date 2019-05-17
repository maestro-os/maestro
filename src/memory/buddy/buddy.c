#include "buddy.h"
#include "../../idt/idt.h"

static block_order_t max_order;
static size_t buddy_size;
static block_state_t *states;

// TODO Free list

static spinlock_t spinlock;

__attribute__((hot))
static inline void lock()
{
	// TODO fix
	/*idt_set_state(false);
	spin_lock(&spinlock);*/
}

__attribute__((hot))
static inline void unlock()
{
	// TODO fix
	/*spin_unlock(&spinlock);
	idt_set_state(true);*/
}

__attribute__((hot))
static block_order_t buddy_get_order(const size_t size)
{
	block_order_t order = 0;
	size_t i = PAGE_SIZE;

	while(i < size)
	{
		++order;
		i <<= 1;
	}

	return order;
}

__attribute__((hot))
static void set_block_state_(const size_t index)
{
	const block_state_t left_state = states[NODE_LEFT(index)];
	const block_state_t right_state = states[NODE_RIGHT(index)];

	if(left_state == NODE_STATE_FREE && right_state == NODE_STATE_FREE)
		states[index] = NODE_STATE_FREE;
	else if(left_state == NODE_STATE_FULL && right_state == NODE_STATE_FULL)
		states[index] = NODE_STATE_FULL;
	else
		states[index] = NODE_STATE_PARTIAL;

	if(NODE_ORDER(max_order, index) < max_order)
		set_block_state_(NODE_PARENT(index));
}

__attribute__((hot))
static void set_block_state(const size_t index, const block_state_t state)
{
	states[index] = state;

	if(NODE_ORDER(max_order, index) < max_order)
		set_block_state_(NODE_PARENT(index));
}

__attribute__((cold))
void buddy_init()
{
	max_order = buddy_get_order(available_memory);
	buddy_size = BLOCK_SIZE(max_order); // TODO Take metadata into account
	states = heap_begin;

	const size_t metadata_size = METADATA_SIZE(max_order);
	bzero(states, metadata_size);
	buddy_begin = ALIGN_UP(states + metadata_size, PAGE_SIZE);

	void *buddy_end = ALIGN_DOWN(heap_end, PAGE_SIZE);
	const size_t end_begin = NODES_COUNT(max_order - 1)
		+ ((uintptr_t) buddy_end / PAGE_SIZE);
	const size_t end_end = NODES_COUNT(max_order);

	for(size_t i = end_begin; i < end_end; ++i)
		set_block_state(i, NODE_STATE_FULL);

	// TODO Free list

	spinlock = 0;
}

__attribute__((hot))
static size_t find_free(const size_t index, const block_order_t order,
	const bool is_buddy)
{
	if(order >= max_order) return BLOCK_NULL;

	const block_order_t block_order = NODE_ORDER(max_order, index);
	const block_state_t block_state = states[index];

	if(block_order < order
		|| (block_order == 0 && block_state != NODE_STATE_FREE))
		return BLOCK_NULL;

	switch(block_state)
	{
		case NODE_STATE_FREE:
		{
			if(block_order > order)
				return find_free(NODE_LEFT(index), order, false);
			else if(block_order == order)
				return index;
			else
				return BLOCK_NULL;

			break;
		}

		case NODE_STATE_PARTIAL:
		{
			if(block_order > order)
			{
				size_t i;

				if((i = find_free(NODE_LEFT(index),
					order, false)) != BLOCK_NULL)
					return i;
				else if((i = find_free(NODE_RIGHT(index),
					order, false)) != BLOCK_NULL)
					return i;
			}
			else if(block_order < order)
				return BLOCK_NULL;

			break;
		}

		case NODE_STATE_FULL: break;
	}

	if(block_order < max_order && !is_buddy)
		return find_free(NODE_BUDDY(index), order, true);
	else
		return BLOCK_NULL;
}

__attribute__((hot))
void *buddy_alloc(const size_t order)
{
	lock();

	// TODO Check free list

	const size_t block = find_free(0, order, false);
	void *ptr = NODE_PTR(buddy_begin, max_order, block);

	if(block != BLOCK_NULL)
		set_block_state(block, NODE_STATE_FULL);

	unlock(); // TODO Doing NODE_PTR after unlock gives a bad value from NODE_PTR
	return ptr;
}

void *buddy_alloc_zero(const size_t order)
{
	void *ptr = buddy_alloc(order);
	bzero(ptr, BLOCK_SIZE(order));

	return ptr;
}

// TODO Fix
__attribute__((hot))
void buddy_free(void *ptr)
{
	lock();

	size_t index = NODES_COUNT(max_order - 1) + ((uintptr_t) ptr / PAGE_SIZE);
	size_t order = 0;

	while(order < max_order && states[index] != NODE_STATE_FULL)
	{
		index = NODE_PARENT(index);
		++order;
	}

	set_block_state(index, NODE_STATE_FREE);
	// TODO Add to free list if necessary

	unlock();
}
