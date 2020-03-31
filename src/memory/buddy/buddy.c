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
#define debug_check_order(order)	debug_assert(order <= BUDDY_MAX_ORDER,\
	"buddy: invalid order")

#define GET_BUDDY_FREE_BLOCK(node)\
	CONTAINER_OF(node, buddy_free_block_t, free_list)

/*
 * This files handles the buddy allocator which allows to allocate 2^^n pages
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
static buddy_free_block_t *get_buddy(void *ptr, const block_order_t order)
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
 * Links a free block for the given pointer with the given order.
 * The block must not already be linked.
 */
static void link_free_block(buddy_free_block_t *ptr, const block_order_t order)
{
	debug_check_block(ptr);
	debug_check_order(order);
	list_insert_front(&free_list[order], &ptr->free_list);
	ptr->node.value = (avl_value_t) ptr;
	ptr->order = order;
	avl_tree_insert(&free_tree, &ptr->node, ptr_cmp);
}

/*
 * Unlinks the given block from the free list and free tree.
 */
static void unlink_free_block(buddy_free_block_t *block)
{
	debug_check_block(block);
	debug_check_order(block->order);
	list_remove(&free_list[block->order], &block->free_list);
	avl_tree_remove(&free_tree, &block->node);
}

/*
 * Splits the given block until a block of the required order is created and
 * returns it.
 * The input block will be unlinked and the newly created blocks will be
 * inserted into the free list and free tree except the returned block.
 */
static buddy_free_block_t *split_block(buddy_free_block_t *block,
	const block_order_t order)
{
	debug_check_block(block);
	debug_check_order(order);
	debug_assert(block->order >= order, "split_block: block too small");
	unlink_free_block(block);
	while(block->order > order)
	{
		--block->order;
		link_free_block((void *) block + BLOCK_SIZE(block->order),
			block->order);
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
	block_order_t order;

	i = mem_info.heap_begin;
	while(i < mem_info.heap_end)
	{
		order = MIN(buddy_get_order((mem_info.heap_end - i) / PAGE_SIZE),
			BUDDY_MAX_ORDER);
		link_free_block(i, order);
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
	if(!free_list[i])
	{
		spin_unlock(&spinlock);
		errno = ENOMEM;
		return NULL;
	}
	debug_assert(GET_BUDDY_FREE_BLOCK(free_list[i])->order == i,
		"buddy_alloc: invalid free list");
	ptr = split_block(GET_BUDDY_FREE_BLOCK(free_list[i]), order);
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
	void *buddy;

	debug_check_block(ptr);
	assert(order <= BUDDY_MAX_ORDER, "buddy_free: order > BUDDY_MAX_ORDER");
	spin_lock(&spinlock);
	link_free_block(ptr, order);
	while(order < BUDDY_MAX_ORDER && (buddy = get_buddy(ptr, order)))
	{
		if(buddy < ptr)
			swap_ptr(&ptr, &buddy);
		unlink_free_block(ptr);
		unlink_free_block(buddy);
		((buddy_free_block_t *) ptr)->order = ++order;
		link_free_block(ptr, order);
	}
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
