#ifndef BUDDY_H
# define BUDDY_H

# include "../memory.h"

# define BLOCK_NULL					(~((size_t) 0))
# define BLOCK_SIZE(order)			(POW2(order) * PAGE_SIZE)

# define NODES_COUNT(order)				(POW2((order) + 1) - 1)
# define METADATA_SIZE(max_order)		(NODES_COUNT(max_order)\
	* sizeof(block_state_t))
# define NODE_ORDER(max_order, node)	((max_order) - floor_log2((node) + 1))
# define NODE_PARENT(node)				(((node) + 1) / 2 - 1)
# define NODE_LEFT(node)				((node) * 2 + 1)
# define NODE_RIGHT(node)				((node) * 2 + 2)
# define NODE_BUDDY(node)				(((node) & 1) == 0\
	? (node) - 1 : (node) + 1)

// TODO Try to optimize
# define NODE_LOCATION(index)			((((index) + 1)\
	% (POW2(floor_log2((index) + 1)))))
# define NODE_PTR(buddy_begin, max_order, index)\
	((index) == BLOCK_NULL ? NULL : ((buddy_begin) + (NODE_LOCATION(index)\
		* BLOCK_SIZE(NODE_ORDER(max_order, index)))))

# define NODE_STATE_FREE	0
# define NODE_STATE_PARTIAL	1
# define NODE_STATE_FULL	2

typedef uint8_t block_state_t;

void *buddy_begin;

void buddy_init(void);

#endif
