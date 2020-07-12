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

/*
 * Pointer to the region of memory containing frames states.
 */
static frame_state_t *frames_states;

/*
 * The total number of allocatable pages.
 */
static size_t pages_count;

/*
 * Pointer to the beginning of buddy memory.
 */
static void *buddy_begin;

/*
 * The list of linked lists containing free frames, sorted according to frames'
 * order.
 */
static frame_state_t *free_list[BUDDY_MAX_ORDER + 1];

/*
 * The spinlock used for buddy allocator operations.
 */
static spinlock_t spinlock = 0;

/*
 * Total number of allocated pages.
 */
size_t total_allocated_pages = 0;

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
 * Links the given element to the given free list.
 */
ATTR_HOT
static void free_list_push(frame_order_t order, frame_state_t *s)
{
	debug_assert(order <= BUDDY_MAX_ORDER && s, "buddy: invalid arguments");
	s->prev = FRAME_ID(s);
	s->next = (free_list[order] ? FRAME_ID(free_list[order]) : FRAME_ID(s));
	s->order = order;
	FRAME_STATE_GET(s->next)->prev = FRAME_ID(s);
	debug_assert(FRAME_STATE_GET(s->prev) == s, "buddy: free list failure");
	debug_check_free_frame(s);
	free_list[order] = s;
}

/*
 * Unlinks the first element of the given free list.
 */
ATTR_HOT
static void free_list_pop(frame_order_t order)
{
	frame_state_t *s;
	size_t frame_id;

	debug_assert(order <= BUDDY_MAX_ORDER, "buddy: invalid argument");
	s = free_list[order];
	debug_check_free_frame(s);
	free_list[order] = FRAME_STATE_GET(s->next);
	frame_id = FRAME_ID(s);
	FRAME_STATE_GET(s->prev)->next = (s->next != frame_id ? s->next : s->prev);
	FRAME_STATE_GET(s->next)->prev = (s->prev != frame_id ? s->prev : s->next);
	s->prev = FRAME_STATE_USED;
	if(FRAME_IS_USED(free_list[order]))
		free_list[order] = NULL;
}

/*
 * Unlinks the given element of the given free list.
 */
ATTR_HOT
static void free_list_remove(frame_order_t order, frame_state_t *s)
{
	size_t frame_id;

	debug_assert(order <= BUDDY_MAX_ORDER && s, "buddy: invalid arguments");
	debug_assert(free_list[order], "buddy: empty free list");
	debug_check_free_frame(s);
	frame_id = FRAME_ID(s);
	if(free_list[order] == s)
	{
		debug_assert(s->prev == frame_id, "buddy: invalid free list");
		free_list[order] = (s->next == frame_id ? NULL
			: FRAME_STATE_GET(s->next));
	}
	FRAME_STATE_GET(s->prev)->next = (s->next != frame_id ? s->next : s->prev);
	FRAME_STATE_GET(s->next)->prev = (s->prev != frame_id ? s->prev : s->next);
	s->prev = FRAME_STATE_USED;
}

/*
 * Initializes the buddy allocator.
 */
ATTR_COLD
void buddy_init(void)
{
	size_t allocatable_memory;
	size_t i, order;
	frame_state_t *s;

	frames_states = mem_info.heap_begin;
	pages_count = (mem_info.heap_end - mem_info.heap_begin)
		/ (PAGE_SIZE + sizeof(frame_state_t));
	buddy_begin = mem_info.heap_begin + pages_count * sizeof(frame_state_t);
	buddy_begin = ALIGN(buddy_begin, PAGE_SIZE);
	debug_assert(buddy_begin + pages_count * PAGE_SIZE == mem_info.heap_end,
		"buddy: invalid allocator memory");
	memset((void *) frames_states, FRAME_STATE_USED,
		pages_count * sizeof(frame_state_t));
	bzero(free_list, sizeof(free_list));
	allocatable_memory = mem_info.heap_end - buddy_begin;
	for(i = 0, order = BUDDY_MAX_ORDER;
		i < allocatable_memory; i += FRAME_SIZE(order))
	{
		while(order > 0 && i + FRAME_SIZE(order) > allocatable_memory)
			--order;
		if(i + FRAME_SIZE(0) > allocatable_memory)
			break;
		s = FRAME_STATE_GET(i / PAGE_SIZE);
		debug_assert((uintptr_t) s < (uintptr_t) buddy_begin,
			"buddy: frame state out of bounds");
		free_list_push(order, s);
	}
}

/*
 * Splits the given frame from order `from` to order `to`.
 */
static void free_list_split(const frame_order_t from, const frame_order_t to)
{
	frame_state_t *s;
	size_t frame_id, i;

	debug_assert(from <= BUDDY_MAX_ORDER && to < from, "buddy: invalid orders");
	s = free_list[from];
	frame_id = FRAME_ID(s);
	free_list_pop(from);
	free_list_push(to, s);
	for(i = to; i < from; ++i)
		free_list_push(i, FRAME_STATE_GET(GET_BUDDY(frame_id, i)));
}

/*
 * Coalesces the given frame from the given order.
 */
static void free_list_coalesce(frame_state_t *b, const frame_order_t order)
{
	size_t i, frame_id, buddy;
	frame_state_t *buddy_state;

	debug_assert(b, "buddy: invalid argument");
	debug_assert(order <= BUDDY_MAX_ORDER, "buddy: invalid order");
	i = order;
	while(i < BUDDY_MAX_ORDER)
	{
		debug_assert(!FRAME_IS_USED(b), "buddy: trying to coalesce used frame");
		frame_id = FRAME_ID(b);
		if(frame_id >= pages_count || frame_id + POW2(i) > pages_count)
			break;
		buddy = GET_BUDDY(frame_id, i);
		if(buddy >= pages_count || buddy + POW2(i) > pages_count)
			break;
		buddy_state = FRAME_STATE_GET(buddy);
		if(buddy_state->order != i)
			break;
		debug_assert(b != buddy_state, "buddy: invalid buddy");
		if(FRAME_IS_USED(buddy_state))
			break;
		free_list_remove(i, b);
		free_list_remove(i, buddy_state);
		b = MIN(b, buddy_state);
		free_list_push(++i, b);
	}
}

/*
 * Allocates a frame of memory using the buddy allocator.
 */
ATTR_HOT
ATTR_MALLOC
void *buddy_alloc(const frame_order_t order)
{
	size_t i;
	void *ptr;

	errno = 0;
	if(order > BUDDY_MAX_ORDER)
		return NULL;
	spin_lock(&spinlock);
	i = order;
	while(i <= BUDDY_MAX_ORDER && !free_list[i])
		++i;
	if(i > BUDDY_MAX_ORDER)
	{
		ptr = NULL;
		goto end;
	}
	if(i != order)
	{
		free_list_split(i, order);
		i = order;
		debug_assert(free_list[i], "Buddy frame split fail");
	}
	ptr = FRAME_ID_PTR(FRAME_ID(free_list[i]));
	free_list_pop(i);
	total_allocated_pages += FRAME_SIZE(order);
	debug_check_frame(buddy_begin, ptr, order);

end:
	spin_unlock(&spinlock);
	return ptr;
}

/*
 * Uses `buddy_alloc` and applies `bzero` on the allocated frame.
 */
ATTR_HOT
ATTR_MALLOC
void *buddy_alloc_zero(const frame_order_t order)
{
	void *ptr;

	if((ptr = buddy_alloc(order)))
		bzero(ptr, FRAME_SIZE(order));
	return ptr;
}

/*
 * Frees the given memory frame that was allocated using the buddy allocator.
 * The given order must be the same as the one given to allocate the frame.
 */
ATTR_HOT
void buddy_free(void *ptr, const frame_order_t order)
{
	frame_state_t *state;

	debug_check_frame(buddy_begin, ptr, order);
	debug_assert(order <= BUDDY_MAX_ORDER,
		"buddy_free: order > BUDDY_MAX_ORDER");
	spin_lock(&spinlock);
	state = FRAME_STATE_GET(FRAME_PTR_ID(ptr));
	debug_assert(FRAME_IS_USED(state), "buddy: freeing unused frame");
	free_list_push(order, state);
	free_list_coalesce(state, order);
	total_allocated_pages -= FRAME_SIZE(order);
	spin_unlock(&spinlock);
}

/*
 * Returns the total number of pages allocated by the buddy allocator.
 */
ATTR_HOT
size_t allocated_pages(void)
{
	return total_allocated_pages;
}

#ifdef KERNEL_DEBUG
/*
 * Asserts that every elements in the free list are valid.
 */
void buddy_free_list_check(void)
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
}

/*
 * Prints the free list.
 */
void buddy_print_free_list(void)
{
	// TODO
}

/*
 * Checks if the given `frame` is in the free list.
 */
int buddy_free_list_has(frame_state_t *state)
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
}
# endif
