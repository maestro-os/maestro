#include <memory/buddy/buddy.h>
#include <memory/buddy/buddy_internal.h>
#include <kernel.h>
#include <idt/idt.h>
#include <debug/debug.h>

#include <libc/errno.h>

/*
 * This files contains the buddy allocator which allows to allocate 2^^n pages
 * large frames of memory.
 *
 * This allocator works by dividing frames of memory in two until the a frame of
 * the required size is available.
 *
 * The order of a frame is the `n` in the expression `2^^n` that represents the
 * size of a frame in pages.
 */

// TODO OOM killer

/*
 * The spinlock used for buddy allocator operations.
 */
static spinlock_t spinlock = 0;

/*
 * Returns the buddy order required to fit the given number of pages.
 */
ATTR_HOT
ATTR_CONST
frame_order_t buddy_get_order(const size_t pages)
{
	frame_order_t order = 0;
	size_t i = 1;

	while(i < pages)
	{
		i *= 2;
		++order;
	}
	return order;
}

/*
 * Initializes the buddy allocator.
 */
ATTR_COLD
void buddy_init(void)
{
	zone_t *kernel_zone;
	size_t pages;
	void *begin;

	kernel_zone = PROCESS_END + (uintptr_t) mem_info.phys_alloc_begin;
	// TODO Subtract DMA and user
	pages = (mem_info.phys_alloc_end - mem_info.phys_alloc_begin)
		/ (PAGE_SIZE + sizeof(frame_state_t));
	begin = ALIGN(mem_info.phys_alloc_begin
		+ pages * sizeof(frame_state_t), PAGE_SIZE);
	zone_init(kernel_zone, BUDDY_FLAG_ZONE_KERNEL, begin, pages);
	// TODO Init other zones
}

/*
 * Allocates a frame of memory using the buddy allocator.
 */
ATTR_HOT
ATTR_MALLOC
void *buddy_alloc(const frame_order_t order, int flags)
{
	void *ptr;
	zone_t *z;
	size_t i;

	errno = 0;
	if(order > BUDDY_MAX_ORDER)
		return NULL;
	ptr = NULL;
	spin_lock(&spinlock);
	if(!(z = zone_get(order, flags & 0b11)))
		goto end;
	i = order;
	while(i <= BUDDY_MAX_ORDER && !z->free_list[i])
		++i;
	if(i > BUDDY_MAX_ORDER)
		goto end;
	if(i != order)
	{
		free_list_split(z, i, order);
		i = order;
		debug_assert(z->free_list[i], "Buddy frame split fail");
	}
	ptr = FRAME_ID_PTR(z, FRAME_ID(z, z->free_list[i]));
	free_list_pop(z, i);
	z->available -= POW2(order);
	debug_check_frame(z, ptr, order);

end:
	spin_unlock(&spinlock);
	return ptr;
}

/*
 * Uses `buddy_alloc` and applies `bzero` on the allocated frame.
 */
ATTR_HOT
ATTR_MALLOC
void *buddy_alloc_zero(const frame_order_t order, int flags)
{
	void *ptr;

	if((ptr = buddy_alloc(order, flags)))
		bzero(ptr, BUDDY_FRAME_SIZE(order));
	return ptr;
}

/*
 * Frees the given memory frame that was allocated using the buddy allocator.
 * The given order must be the same as the one given to allocate the frame.
 */
ATTR_HOT
void buddy_free(void *ptr, const frame_order_t order)
{
	zone_t *z;
	frame_state_t *state;

	z = zone_get_for(ptr);
	debug_assert(sanity_check(z), "buddy: internal error");
	debug_check_frame(z, ptr, order);
	debug_assert(order <= BUDDY_MAX_ORDER, "buddy: order > BUDDY_MAX_ORDER");
	spin_lock(&spinlock);
	state = &z->states[FRAME_PTR_ID(z, ptr)];
	debug_assert(FRAME_IS_USED(state), "buddy: freeing unused frame");
	free_list_push(z, order, state);
	free_list_coalesce(z, state, order);
	z->available += POW2(order);
	spin_unlock(&spinlock);
}

/*
 * Returns the total number of pages allocated by the buddy allocator.
 */
ATTR_HOT
size_t allocated_pages(void)
{
	// TODO
	return 0;
}

#ifdef KERNEL_DEBUG
/*
 * Asserts that every elements in the free list are valid.
 */
/*void buddy_free_list_check(void)
{
	size_t i, j;
	frame_state_t *s, *next;

	//printf("--- Buddy free list check ---\n");
	for(i = 0; i <= BUDDY_MAX_ORDER; ++i)
	{
		if(!(s = free_list[i]))
			continue;
		j = 0;
		while(1)
		{
			//printf("-> order: %zu element: %zu\n", i, j);
			debug_check_free_frame(s);
			next = FRAME_STATE_GET(s->next);
			if(next == s)
				break;
			s = next;
			++j;
		}
	}
}*/

/*
 * Prints the free list.
 */
/*void buddy_print_free_list(void)
{
	// TODO
}*/

/*
 * Checks if the given `frame` is in the free list.
 */
/*int buddy_free_list_has(frame_state_t *state)
{
	size_t i;

	for(i = 0; i <= BUDDY_MAX_ORDER; ++i)
		if(buddy_free_list_has_(state, i))
			return 1;
	return 0;
}

int buddy_free_list_has_(frame_state_t *state, frame_order_t order)
{
	frame_state_t *s, *next;

	if(!(s = free_list[order]))
		return 0;
	while(1)
	{
		debug_check_free_frame(s);
		if(s == state)
			return 1;
		next = FRAME_STATE_GET(s->next);
		if(next == s)
			break;
		s = next;
	}
	return 0;
}*/
# endif
