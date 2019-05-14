#ifndef BUDDY_H
# define BUDDY_H

# include "../memory.h"

# define BLOCK_NULL					(~((size_t) 0))
# define BLOCK_SIZE(order)			(POW2(order) * PAGE_SIZE)

# define NODES_COUNT(max_order)			(POW2((max_order) + 1) - 1)
# define METADATA_SIZE(max_order)		(NODES_COUNT(max_order)\
	* sizeof(block_state_t))
# define NODE_ORDER(max_order, node)	((max_order) - ulog2((node) + 1))
# define NODE_PARENT(node)				(((node) + 1) / 2 - 1)
# define NODE_LEFT(node)				((node) * 2 + 1)
# define NODE_RIGHT(node)				((node) * 2 + 2)
# define NODE_BUDDY(node)				(((node) & 1) == 0\
	? (node) - 1 : (node) + 1)
# define NODE_LOCATION(index)			((((index) + 1)\
	% (1 << ulog2((index) + 1))))
# define NODE_PTR(index)				((index) != BLOCK_NULL\
	? (void *) buddy_begin + (NODE_LOCATION(index) * PAGE_SIZE) : NULL)

# define NODE_STATE_FREE	0
# define NODE_STATE_PARTIAL	1
# define NODE_STATE_FULL	2

typedef uint32_t block_order_t;
typedef uint32_t block_state_t;

void buddy_init();

#endif
