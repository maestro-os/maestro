#ifndef BUDDY_H
# define BUDDY_H

# include "../memory.h"

# define BLOCK_NULL					(~((size_t) 0))
# define BLOCK_SIZE(order)			(POW2(order) * PAGE_SIZE)
# define BLOCK_PTR(index)			((i != BUDDY_NULL)\
	? (void *) buddy_begin + ((i) * PAGE_SIZE) : NULL)
# define BLOCK_BUDDY(index, size)	((index) ^ (size))

# define BUDDY_STATE(order, use)	(((order) << 1) | (use & 1))
# define BUDDY_STATE_ORDER(state)	((state) >> 1)
# define BUDDY_STATE_USE(state)		((state) & 1)

typedef uint32_t buddy_order_t;
typedef uint32_t buddy_state_t;

void *buddy_begin;

void buddy_init();

#endif
