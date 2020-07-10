#include <memory/buddy/buddy.h>
#include <memory/buddy/buddy_internal.h>
#include <kernel.h>
#include <idt/idt.h>

#include <libc/errno.h>

/*
 * This files contains the buddy allocator which allows to allocate 2^^n pages
 * large blocks of memory.
 *
 * This allocator works by dividing blocks of memory in two until the a block of
 * the required size is available.
 *
 * The order of a block is the `n` in the expression `2^^n` that represents the
 * size of a block in pages.
 */

/*
 * Pointer to the region of memory containing blocks states.
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
 * The list of linked lists containing free blocks, sorted according to blocks'
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
block_order_t buddy_get_order(const size_t pages)
{
	block_order_t order = 0;
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
static void free_list_push(frame_state_t **list, frame_state_t *s)
{
	debug_assert(list && s, "Invalid arguments");
	s->prev = FRAME_ID(s);
	s->next = (*list ? FRAME_ID(*list) : FRAME_ID(s));
	FRAME_STATE_GET(s->next)->prev = FRAME_ID(s);
	debug_assert(FRAME_STATE_GET(s->prev) == s, "buddy: free list failure");
	debug_check_free_frame(s);
	*list = s;
#ifdef KERNEL_DEBUG
	/*debug_assert(buddy_free_list_has(FRAME_ID_PTR(FRAME_ID(s))),
		"buddy: free list push failed");*/
#endif
}

/*
 * Unlinks the first element of the given free list.
 */
ATTR_HOT
static void free_list_pop(frame_state_t **list)
{
	frame_state_t *s;
	size_t frame_id;

	debug_assert(list && *list, "Invalid argument");
	s = *list;
	debug_check_free_frame(s);
	*list = FRAME_STATE_GET(s->next);
	frame_id = FRAME_ID(s);
	FRAME_STATE_GET(s->prev)->next = (s->next != frame_id ? s->next : s->prev);
	FRAME_STATE_GET(s->next)->prev = (s->prev != frame_id ? s->prev : s->next);
	s->prev = FRAME_STATE_USED;
	s->next = FRAME_STATE_USED;
	if(FRAME_IS_USED(*list))
		*list = NULL;
#ifdef KERNEL_DEBUG
	//buddy_free_list_check();
#endif
}

/*
 * Unlinks the given element of the given free list.
 */
ATTR_HOT
static void free_list_remove(frame_state_t **list, frame_state_t *s)
{
	size_t frame_id;

	debug_assert(list && *list && s, "Invalid arguments");
	debug_check_free_frame(s);
	frame_id = FRAME_ID(s);
	if(*list == s)
		*list = (s->next == frame_id ? NULL : FRAME_STATE_GET(s->next));
	FRAME_STATE_GET(s->prev)->next = (s->next != frame_id ? s->next : s->prev);
	FRAME_STATE_GET(s->next)->prev = (s->prev != frame_id ? s->prev : s->next);
	s->prev = FRAME_STATE_USED;
	s->next = FRAME_STATE_USED;
#ifdef KERNEL_DEBUG
	/*debug_assert(!buddy_free_list_has(FRAME_ID_PTR(FRAME_ID(s))),
		"buddy: free list remove failed");*/
#endif
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
		"Invalid buddy allocator memory");
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
		free_list_push(&free_list[order], s);
	}
}

/*
 * Splits the given block from order `from` to order `to`.
 */
static void free_list_split(const block_order_t from, const block_order_t to)
{
	size_t i;
	frame_state_t *s, *buddy;

	debug_assert(from <= BUDDY_MAX_ORDER && to < from, "Invalid orders");
	for(i = from; i > to; --i)
	{
		s = free_list[i];
		buddy = FRAME_STATE_GET(GET_BUDDY(FRAME_ID(s), i - 1));
		free_list_pop(&free_list[i]);
		free_list_push(&free_list[i - 1], s);
		free_list_push(&free_list[i - 1], buddy);
	}
}

/*
 * Coalesces the given block from the given order.
 */
static void free_list_coalesce(frame_state_t *b, const block_order_t order)
{
	size_t i, buddy;
	frame_state_t *buddy_state;

	debug_assert(b, "Invalid argument");
	debug_assert(order <= BUDDY_MAX_ORDER, "Invalid order");
	i = order;
	while(i < BUDDY_MAX_ORDER)
	{
		debug_assert(!FRAME_IS_USED(b), "buddy: trying to coalesce used block");
		if((buddy = GET_BUDDY(FRAME_ID(b), i)) + POW2(i) >= pages_count)
			break;
		buddy_state = FRAME_STATE_GET(buddy);
		debug_assert(b != buddy_state, "buddy: invalid buddy");
		if(FRAME_IS_USED(buddy_state))
			break;
		free_list_remove(&free_list[i], b);
		free_list_remove(&free_list[i], buddy_state);
		b = MIN(b, buddy_state);
		free_list_push(&free_list[++i], b);
	}
}

/*
 * Allocates a block of memory using the buddy allocator.
 */
ATTR_HOT
ATTR_MALLOC
void *buddy_alloc(const block_order_t order)
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
		debug_assert(free_list[i], "Buddy block split fail");
	}
	ptr = FRAME_ID_PTR(FRAME_ID(free_list[i]));
	free_list_pop(&free_list[i]);
	total_allocated_pages += FRAME_SIZE(order);

end:
	spin_unlock(&spinlock);
	return ptr;
}

/*
 * Uses `buddy_alloc` and applies `bzero` on the allocated block.
 */
ATTR_HOT
ATTR_MALLOC
void *buddy_alloc_zero(const block_order_t order)
{
	void *ptr;

	if((ptr = buddy_alloc(order)))
		bzero(ptr, FRAME_SIZE(order));
	return ptr;
}

/*
 * Frees the given memory block that was allocated using the buddy allocator.
 * The given order must be the same as the one given to allocate the block.
 */
ATTR_HOT
void buddy_free(void *ptr, const block_order_t order)
{
	frame_state_t *state;

	debug_check_block(buddy_begin, ptr, order);
	debug_assert(order <= BUDDY_MAX_ORDER,
		"buddy_free: order > BUDDY_MAX_ORDER");
	spin_lock(&spinlock);
	state = FRAME_STATE_GET(FRAME_PTR_ID(ptr));
	debug_assert(FRAME_IS_USED(state), "Freeing unused block");
	free_list_push(&free_list[order], state);
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
 * Checks if the given `ptr` is in the free list.
 */
int buddy_free_list_has(void *ptr)
{
	size_t frame_id;
	size_t i;
	frame_state_t *s, *next;

	frame_id = FRAME_PTR_ID(ptr);
	for(i = 0; i <= BUDDY_MAX_ORDER; ++i)
	{
		if(!(s = free_list[i]))
			continue;
		while(1)
		{
			debug_check_free_frame(s);
			if(FRAME_ID(s) == frame_id)
				return 1;
			next = FRAME_STATE_GET(s->next);
			if(next == s)
				break;
			s = next;
		}
	}
	return 0;
}
# endif
