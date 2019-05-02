#ifndef BUDDY_H
# define BUDDY_H

# include "../util/util.h"

# define MAX_BUDDY_NODES(order)		(POW2((order) + 1) - 1)
// TODO Maybe use more than one bit for each buddy?
# define BUDDY_STATES_SIZE(size)	(UPPER_DIVISION((size), 8))

# define ORDERTOSIZE(order)	(BLOCK_SIZE * POW2(order))
# define SIZETOORDER(size)	((size) / BLOCK_SIZE)

# define BUDDY_INDEX(max_order, order, i)	((max_order) - (order) + (i))
# define BUDDY_PARENT(i)					(((i) + 1) / 2 - 1)

# define HEAP_BEGIN_VAL	0x400000
# define HEAP_BEGIN		((void *) HEAP_BEGIN_VAL)

typedef uint16_t buddy_order_t;

buddy_order_t buddy_get_order(const size_t size);

#endif
