#ifndef BUDDY_H
# define BUDDY_H

# include <libc/string.h>
# include <memory/memory.h>
# include <util/util.h>

# define BUDDY_MAX_ORDER	17

# define BLOCK_SIZE(order)	(PAGE_SIZE << (order))
# define MAX_BLOCK_SIZE		(PAGE_SIZE << BUDDY_MAX_ORDER)

typedef unsigned block_order_t;

block_order_t buddy_get_order(size_t pages);
void buddy_init(void);

ATTR_MALLOC
void *buddy_alloc(block_order_t order);
ATTR_MALLOC
void *buddy_alloc_zero(block_order_t order);
void buddy_free(void *ptr, block_order_t order);

size_t allocated_pages(void);

#endif
