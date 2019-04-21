#ifndef BUDDY_H
# define BUDDY_H

// TODO Use pages instead of blocks?
# define BLOCK_SIZE		0x10000
# define BUDDY_MIN_SIZE	0x100000

# define MAX_BUDDY_NODES(order)	(POW2((order) + 1) - 1)
// TODO Add informations for every page?
# define ALLOC_META_SIZE(order)	(UPPER_DIVISION(MAX_BUDDY_NODES(order)\
	* sizeof(buddy_state_t), 8))

# define ORDERTOSIZE(order)					(BLOCK_SIZE * pow2(order))
# define BUDDY_INDEX(max_order, order, i)	((max_order) - (order) + (i))
# define BUDDY_PARENT(i)					((i) & 1 == 0 ? (i) : (i) - 1)

# define HEAP_BEGIN_VAL	0x400000
# define HEAP_BEGIN		((void *) HEAP_BEGIN_VAL)

# if HEAP_BEGIN_VAL < BLOCK_SIZE
#  error "BLOCK_SIZE must be lower than HEAP_BEGIN!"
# endif

#endif
