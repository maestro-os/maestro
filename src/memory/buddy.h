#ifndef BUDDY_H
# define BUDDY_H

# include "../util/util.h"

# define BUDDY_STATES_SIZE(size)	(UPPER_DIVISION((size), 8))
# define BUDDY_NULL					((size_t) -1)

# define BUDDY_INDEX(max_order, order, i)	((max_order) - (order) + (i))
# define BUDDY_PTR(order, i)				((i != BUDDY_NULL)\
	? (void *) (POW2(order) * PAGE_SIZE * (i)) : NULL)

# define BLOCKS_COUNT(max_order, order)\
	((size_t) POW2((max_order) - (order)))

# define HEAP_BEGIN_VAL	0x400000
# define HEAP_BEGIN		((void *) HEAP_BEGIN_VAL)

typedef uint16_t buddy_order_t;

buddy_order_t buddy_get_order(const size_t size);
int buddy_get_block(const size_t i, const buddy_order_t order);
void buddy_set_block(const size_t i, const buddy_order_t order, const int used);

#endif
