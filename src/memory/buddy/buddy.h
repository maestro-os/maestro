#ifndef BUDDY_H
# define BUDDY_H

# include <memory/memory.h>

# include <libc/string.h>

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

# define NODE_LOCATION(index)			((((index) + 1)\
	% (POW2(floor_log2((index) + 1)))))
# define NODE_PTR(buddy_begin, max_order, index)\
	((index) < 0 ? NULL : ((buddy_begin) + (NODE_LOCATION(index)\
		* BLOCK_SIZE(NODE_ORDER(max_order, index)))))

# define NODE_STATE_FREE	0
# define NODE_STATE_PARTIAL	1
# define NODE_STATE_FULL	2

typedef int block_index_t;
typedef uint32_t block_order_t;
typedef uint8_t block_state_t;

block_order_t buddy_get_order(size_t size);
void *buddy_get_begin(void);
void buddy_init(void);

void *buddy_alloc(block_order_t order);
void *buddy_alloc_zero(block_order_t order);
void buddy_free(void *ptr);

size_t allocated_pages(void);

#endif
