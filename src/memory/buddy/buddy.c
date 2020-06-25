#include <memory/buddy/buddy.h>
#include <memory/buddy/buddy_internal.h>
#include <kernel.h>
#include <idt/idt.h>

#include <libc/errno.h>

#ifdef KERNEL_DEBUG
# include <debug/debug.h>
#endif

#define debug_check_block(ptr)		debug_assert(sanity_check(ptr)\
	&& IS_ALIGNED((ptr), PAGE_SIZE) && (void *) (ptr) >= mem_info.heap_begin\
		&& (void *) (ptr) < mem_info.heap_end, "buddy: invalid block")
#define debug_check_order(order)	debug_assert((order) <= BUDDY_MAX_ORDER,\
	"buddy: invalid order")

#define GET_BUDDY_FREE_BLOCK(node)\
	CONTAINER_OF(node, buddy_free_block_t, free_list)

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
  * The list of linked lists containing free blocks, sorted according to blocks'
  * order.
  */
ATTR_BSS
static list_head_t *free_list[BUDDY_MAX_ORDER + 1];

/*
 * The tree containing free blocks sorted according to their address.
 */
static avl_tree_t *free_tree = NULL;

/*
 * The spinlock used for buddy allocator operations.
 */
static spinlock_t spinlock = 0;

/*
 * Returns the buddy order required to fit the given number of pages.
 */
ATTR_HOT
block_order_t buddy_get_order(const size_t pages)
{
	block_order_t order = 0;
	size_t i = 1;

	while(i < pages)
	{
		i <<= 1;
		++order;
	}
	return order;
}

/*
 * Returns the given block's buddy.
 * Returns `NULL` if the buddy block is not free.
 */
static buddy_free_block_t *get_buddy(buddy_free_block_t *ptr,
	const block_order_t order)
{
	void *buddy_addr;

	debug_check_block(ptr);
	debug_check_order(order);
	buddy_addr = BUDDY_ADDR(ptr, order);
	debug_check_block(buddy_addr);
	if(!avl_tree_search(free_tree, (avl_value_t) buddy_addr, ptr_cmp))
		return NULL;
	return buddy_addr;
}

/*
 * Links the given block to the free list and free tree.
 * The block must not already be linked.
 */
static void link_free_block(buddy_free_block_t *ptr)
{
	debug_check_block(ptr);
	debug_check_order(ptr->order);
	list_insert_front(&free_list[ptr->order], &ptr->free_list);
	ptr->node.value = (avl_value_t) ptr;
	avl_tree_insert(&free_tree, &ptr->node, ptr_cmp);
}

/*
 * Unlinks the given block from the free list and free tree.
 */
static void unlink_free_block(buddy_free_block_t *ptr)
{
	debug_check_block(ptr);
	debug_check_order(ptr->order);
	list_remove(&free_list[ptr->order], &ptr->free_list);
	avl_tree_remove(&free_tree, &ptr->node);
}

/*
 * Splits the given block until a block of the required order is created and
 * returns it.
 * The input block will be unlinked and the newly created blocks will be
 * inserted into the free list and free tree except the returned block.
 */
static void *split_block(buddy_free_block_t *block, const block_order_t order)
{
	size_t i;
	buddy_free_block_t *new;

	debug_check_block(block);
	debug_check_order(order);
	debug_assert(block->order >= order, "split_block: block too small");
	i = block->order;
	unlink_free_block(block);
	while(i > order)
	{
		--i;
		new = (void *) block + BLOCK_SIZE(i);
		new->order = i;
		link_free_block(new);
	}
	return block;
}

/*
 * Initializes the buddy allocator.
 */
ATTR_COLD
void buddy_init(void)
{
	void *i;
	buddy_free_block_t *block;
	block_order_t order;

	i = mem_info.heap_begin;
	while(i + PAGE_SIZE <= mem_info.heap_end)
	{
		order = MIN(buddy_get_order((mem_info.heap_end - i) / PAGE_SIZE),
			BUDDY_MAX_ORDER);
		if(BLOCK_SIZE(order) > (uintptr_t) (mem_info.heap_end - i))
			--order;
		block = i;
		block->order = order;
		link_free_block(block);
		i += BLOCK_SIZE(order);
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
	while(i < BUDDY_MAX_ORDER + 1 && !free_list[i])
		++i;
	if(i >= BUDDY_MAX_ORDER + 1)
	{
		spin_unlock(&spinlock);
		errno = ENOMEM;
		return NULL;
	}
	debug_assert(GET_BUDDY_FREE_BLOCK(free_list[i])->order == i,
		"buddy_alloc: invalid free list");
	ptr = split_block(GET_BUDDY_FREE_BLOCK(free_list[i]), order);
	debug_assert(!buddy_free_list_has(ptr), "buddy_alloc: block not unlinked");
	debug_check_block(ptr);
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
void buddy_free(void *ptr, block_order_t order)
{
	buddy_free_block_t *block, *buddy;

	block = ptr;
	debug_check_block(block);
	debug_assert(order <= BUDDY_MAX_ORDER,
		"buddy_free: order > BUDDY_MAX_ORDER");
	spin_lock(&spinlock);
	block->order = order;
	link_free_block(block);
	while(block->order < BUDDY_MAX_ORDER && (buddy = get_buddy(block, order)))
	{
		if(buddy < block)
			swap_ptr((void **) &block, (void **) &buddy);
		unlink_free_block(block);
		unlink_free_block(buddy);
		++block->order;
		link_free_block(block);
	}
	debug_assert(buddy_free_list_has(ptr), "buddy_free: block not linked");
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

# ifdef KERNEL_DEBUG
/*
 * Prints the free list.
 */
void buddy_print_free_list(void)
{
	size_t i;
	list_head_t *l;

	printf("Free list:\n");
	for(i = 0; i <= BUDDY_MAX_ORDER; ++i)
	{
		printf("- %zu: ", i);
		l = free_list[i];
		while(l)
		{
			printf("%p", CONTAINER_OF(l, buddy_free_block_t, free_list));
			if((l = l->next))
				printf(" ");
		}
		printf("\n");
	}
}

/*
 * Checks if the given `ptr` is in the free list.
 */
int buddy_free_list_has(void *ptr)
{
	size_t i;
	list_head_t *l;
	buddy_free_block_t *b;

	for(i = 0; i <= BUDDY_MAX_ORDER; ++i)
	{
		l = free_list[i];
		while(l)
		{
			b = CONTAINER_OF(l, buddy_free_block_t, free_list);
			if((void *) b <= ptr && (void *) b + BLOCK_SIZE(b->order) > ptr)
				return 1;
			l = l->next;
		}
	}
	return 0;
}
# endif
