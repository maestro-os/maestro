#include <memory/buddy/buddy.h>
#include <memory/buddy/buddy_internal.h>
#include <kernel.h>
#include <idt/idt.h>

#include <libc/errno.h>

#ifdef KERNEL_DEBUG
# include <debug/debug.h>

# define debug_check_block(begin, ptr, order)\
	debug_assert(sanity_check(ptr)\
		&& IS_ALIGNED((ptr) - (begin), PAGE_SIZE << (order))\
		&& (void *) (ptr) >= mem_info.heap_begin\
		&& (void *) (ptr) < mem_info.heap_end, "buddy: invalid block")
# define debug_check_order(order)\
	debug_assert((order) <= BUDDY_MAX_ORDER, "buddy: invalid order")
#else
# define debug_check_block(ptr)
# define debug_check_order(order)
#endif

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
static block_state_t *blocks_states;

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
static block_state_t *free_list[BUDDY_MAX_ORDER + 1];

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
 * Initializes the buddy allocator.
 */
ATTR_COLD
void buddy_init(void)
{
	size_t allocatable_memory;
	size_t i, order;

	blocks_states = mem_info.heap_begin;
	pages_count = (mem_info.heap_end - mem_info.heap_begin)
		/ (PAGE_SIZE + sizeof(block_state_t));
	buddy_begin = mem_info.heap_begin + pages_count * sizeof(block_state_t);
	buddy_begin = ALIGN(buddy_begin, PAGE_SIZE);
	debug_assert(buddy_begin + pages_count * PAGE_SIZE == mem_info.heap_end,
		"Invalid buddy allocator memory");
	memset(blocks_states, BLOCK_STATE_USED,
		pages_count * sizeof(block_state_t));
	bzero(free_list, sizeof(free_list));
	allocatable_memory = mem_info.heap_end - buddy_begin;
	for(i = 0, order = BUDDY_MAX_ORDER;
		i < allocatable_memory; i += BLOCK_SIZE(order))
	{
		while(order > 0 && i + BLOCK_SIZE(order) > allocatable_memory)
			--order;
		if(i + BLOCK_SIZE(0) > allocatable_memory)
			break;
		if(free_list[order])
			blocks_states[i / PAGE_SIZE] = *free_list[order];
		free_list[order] = &blocks_states[i / PAGE_SIZE];
	}
}

/*
 * Links the given element to the given free list.
 */
ATTR_HOT
static void free_list_push(block_state_t **list, block_state_t *b)
{
	debug_assert(list && b, "Invalid arguments");
	if(*list)
	{
		*b = **list;
		*list = b;
	}
	else
	{
		*b = BLOCK_ID(b);
		*list = b;
	}
}

/*
 * Unlinks the first element of the given free list.
 */
ATTR_HOT
static void free_list_pop(block_state_t **list)
{
	block_state_t *b;

	debug_assert(list && *list, "Invalid argument");
	b = *list;
	*list = &blocks_states[*b];
	*b = BLOCK_STATE_USED;
	if(**list == BLOCK_STATE_USED)
		*list = NULL;
}

/*
 * Splits the given block from order `from` to order `to`.
 */
static void free_list_split(const block_order_t from, const block_order_t to)
{
	size_t i;
	block_state_t *b;

	debug_assert(from <= BUDDY_MAX_ORDER && to < from, "Invalid orders");
	for(i = from; i > to; --i)
	{
		b = free_list[i];
		free_list_pop(&free_list[i]);
		free_list_push(&free_list[i - 1], b);
		free_list_push(&free_list[i - 1],
			&blocks_states[GET_BUDDY(BLOCK_ID(b), i)]);
	}
}

/*
 * Coalesces the given block from the given order.
 */
static void free_list_coalesce(block_state_t *b, const block_order_t order)
{
	size_t i, buddy;

	debug_assert(b, "Invalid argument");
	debug_assert(order <= BUDDY_MAX_ORDER, "Invalid order");
	for(i = order; i <= BUDDY_MAX_ORDER; ++i)
	{
		debug_assert(blocks_states[BLOCK_ID(b)] == BLOCK_STATE_USED,
			"Trying to coalesce used block");
		buddy = GET_BUDDY(BLOCK_ID(b), i);
		if(buddy >= pages_count || blocks_states[buddy] == BLOCK_STATE_USED)
			break;
		// TODO Unlink `b` and `buddy` (buddy might be in the middle of the list)
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
	if(i == order)
	{
		free_list_split(i, order);
		i = order;
		debug_assert(free_list[i], "Buddy block split fail");
	}
	ptr = BLOCK_ID_PTR(BLOCK_ID(free_list[i]));
	free_list_pop(&free_list[i]);
	total_allocated_pages += BLOCK_SIZE(order);

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
		bzero(ptr, BLOCK_SIZE(order));
	return ptr;
}

/*
 * Frees the given memory block that was allocated using the buddy allocator.
 * The given order must be the same as the one given to allocate the block.
 */
ATTR_HOT
void buddy_free(void *ptr, const block_order_t order)
{
	debug_check_block(buddy_begin, ptr, order);
	debug_assert(order <= BUDDY_MAX_ORDER,
		"buddy_free: order > BUDDY_MAX_ORDER");
	spin_lock(&spinlock);
	debug_assert(blocks_states[BLOCK_PTR_ID(ptr)] == BLOCK_STATE_USED,
		"Freeing unused block");
	free_list_push(&free_list[order], &blocks_states[BLOCK_PTR_ID(ptr)]);
	free_list_coalesce(&blocks_states[BLOCK_PTR_ID(ptr)], order);
	total_allocated_pages -= BLOCK_SIZE(order);
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

# ifdef KERNEL_DEBUG
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
	// TODO
	(void) ptr;
	return 0;
}
# endif
