#ifndef BUDDY_H
# define BUDDY_H

# define BLOCK_SIZE		0x10000

# define MAX_BUDDY_NODES(order)	(POW2((order) + 1) - 1)
// TODO Maybe use more than one bit for each buddy?
# define ALLOC_META_SIZE(order)	(UPPER_DIVISION(MAX_BUDDY_NODES(order)\
	* sizeof(buddy_state_t), 8))

# define ORDERTOSIZE(order)					(BLOCK_SIZE * POW2(order))
# define BUDDY_INDEX(max_order, order, i)	((max_order) - (order) + (i))
// TODO
# define BUDDY_PARENT(i)					(0)

# define HEAP_BEGIN_VAL	0x400000
# define HEAP_BEGIN		((void *) HEAP_BEGIN_VAL)

typedef uint8_t buddy_state_t;

#endif
