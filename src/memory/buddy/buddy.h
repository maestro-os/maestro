#ifndef BUDDY_H
# define BUDDY_H

# include <libc/string.h>
# include <memory/memory.h>
# include <util/util.h>

/*
 * The maximum order of a buddy allocated frame.
 */
# define BUDDY_MAX_ORDER	17

/*
 * The size in bytes of a frame allocated by the buddy allocator with the given
 * `order`.
 */
# define BUDDY_FRAME_SIZE(order)	(PAGE_SIZE << (order))

/*
 * Buddy allocator flag. Tells that the allocation shall not fail (unless not
 * enough memory is present on the system). This flag is ignored if
 * BUDDY_FLAG_USER is not specified or if the allocation order is higher than 0.
 * The allocator shall use the OOM killer to recover memory.
 */
# define BUDDY_FLAG_NOFAIL		0b0001
/*
 * Buddy allocator flag. Tells that the allocated frame must be mapped into the
 * user zone.
 */
# define BUDDY_FLAG_ZONE_USER	0b0010
/*
 * Buddy allocator flag. Tells that the allocated frame must be mapped into the
 * kernel zone. If the flag is set and no space is available, the allocation
 * shall fail.
 */
# define BUDDY_FLAG_ZONE_KERNEL	0b0100
/*
 * Buddy allocator flag. Tells that the allocated frame must be mapped into the
 * DMA zone.
 */
# define BUDDY_FLAG_ZONE_DMA	0b1000

typedef uint8_t frame_order_t;

frame_order_t buddy_get_order(size_t pages);
void buddy_init(void);

ATTR_MALLOC
void *buddy_alloc(frame_order_t order, int flags);
ATTR_MALLOC
void *buddy_alloc_zero(frame_order_t order, int flags);
void buddy_free(void *ptr, frame_order_t order);

size_t allocated_pages(void);
#endif
