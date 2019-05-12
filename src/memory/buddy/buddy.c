#include "buddy.h"
#include "../../idt/idt.h"

static buddy_order_t max_order;
static size_t buddy_size;
static buddy_state_t *states;
void *buddy_begin;

// TODO Free list

static spinlock_t spinlock;

__attribute__((hot))
static inline void lock()
{
	idt_set_state(false);
	spin_lock(&spinlock);
}

__attribute__((hot))
static inline void unlock()
{
	spin_unlock(&spinlock);
	idt_set_state(true);
}

__attribute__((hot))
static buddy_order_t buddy_get_order(const size_t size)
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

/*__attribute__((hot))
static void buddy_set_state(const size_t index, const buddy_order_t order,
	const int used)
{
	// TODO
	(void) index;
	(void) order;
	(void) used;

	const size_t size = POW2(order);

	if(used)
	{
		if(order + 1 < max_order)
			buddy_set_state(index, order + 1, 1);

		states[index] = BUDDY_STATE(order, 1);
		states[index ^ size] = BUDDY_STATE(order, 0);
	}
	else
	{
		states[index] = BUDDY_STATE(order, 0);

		if(BUDDY_STATE_USED(states[index]) == 0
			&& BUDDY_STATE_USED(states[index ^ size]) == 0
				&& order < max_order)
		{
			const size_t i = (index < (index ^ size) ? index : (index ^ size));
			buddy_set_state(i, order + 1, 0);
		}
	}
}*/

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
	// TODO Set end blocks used
	(void) buddy_end;

	// TODO Free list

	spinlock = 0;
}

__attribute__((hot))
static inline void split_block(const size_t index)
{
	// TODO
	(void) index;

	/*const size_t order = BUDDY_STATE_ORDER(state[index]);
	state[index] = BUDDY_STATE(order - 1, 0);
	state[index ^ POW2(order)] = BUDDY_STATE(order - 1, 0);*/
}

__attribute__((hot))
static size_t find_free(const size_t index, const buddy_order_t order)
{
	(void) index;
	(void) order;

	/*const buddy_order_t o = BUDDY_STATE_ORDER(state[index]);
	const size_t size = POW2(o);
	const int u = BUDDY_STATE_USE(state[index]);

	if(u)
	{
		if(order == max_order)
			return BUDDY_NULL;

		// Stack overflow?
		return find_free(index ^ size, order);
	}
	else
	{
		// TODO Split until obtaining a block small enough
		// TODO Mark as used
		// TODO Return block
	}*/

	return BLOCK_NULL;
}

__attribute__((hot))
void *buddy_alloc(const size_t order)
{
	lock();

	// TODO
	(void) order;
	(void) find_free;

	// TODO Check free list
	// TODO If no block large enough is in free list, look for a block in buddies

	/*size_t buddy;
	buddy_order_t n = 0;

	do
		buddy = find_buddy(order + n);
	while((buddy == BUDDY_NULL) && (order + ++n < max_order));

	if(buddy == BUDDY_NULL) return NULL;
	if(n == 0) return BUDDY_PTR(order, buddy);
	return BUDDY_PTR(order, split_block(order + n, buddy, n));*/

	unlock();
	return NULL;
}

void *buddy_alloc_zero(const size_t order)
{
	void *ptr = buddy_alloc(order);
	bzero(ptr, BLOCK_SIZE(order));
	return ptr;
}

__attribute__((hot))
void buddy_free(void *ptr)
{
	lock();

	// TODO
	(void) ptr;

	unlock();
}
