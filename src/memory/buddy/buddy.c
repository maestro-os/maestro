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
static buddy_free_block_t *free_list[BUDDY_MAX_ORDER + 1];

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
	if(!avl_tree_search(free_tree, (avl_value_t) buddy_addr, ptr_cmp))
		return NULL;
	debug_check_block(buddy_addr);
	return buddy_addr;
}

/*
 * Returns the AVL node of the nearest free block from the given block.
 */
static avl_tree_t *get_nearest_free_block(const buddy_free_block_t *block)
{
	avl_tree_t *n;

	debug_check_block(block);
	if(!(n = free_tree))
		return NULL;
	void *ebp;
	GET_EBP(ebp);
	print_callstack(ebp, 8);
	printf("-------\n");
	while(n)
	{
		printf("-> %p\n", n);
		static int i = 0;
		if(i++ > 115)
			kernel_halt();
		if(block == (void *) n->value)
			break;
		if(ABS((intptr_t) block - (intptr_t) n->left)
			< ABS((intptr_t) block - (intptr_t) n->right))
			n = n->left;
		else
			n = n->right;
	}
	return n;
}

/*
 * Links a free block for the given pointer with the given order.
 * The block must not be linked yet.
 */
static void link_free_block(buddy_free_block_t *ptr,
	const block_order_t order)
{
	avl_tree_t *n;
	buddy_free_block_t *b;

	debug_check_block(ptr);
	debug_check_order(order);
	ptr->prev_free = NULL;
	if((ptr->next_free = free_list[order]))
		ptr->next_free->prev_free = ptr;
	free_list[order] = ptr;
	if((n = get_nearest_free_block(ptr)))
	{
		b = CONTAINER_OF(n, buddy_free_block_t, node);
		if(b < ptr)
		{
			if((ptr->next = b->next))
				ptr->next->prev = ptr;
			if((ptr->prev = b))
				ptr->prev->next = ptr;
		}
		else
		{
			if((ptr->prev = b->prev))
				ptr->prev->next = ptr;
			if((ptr->next = b))
				ptr->next->prev = ptr;
		}
	}
	else
	{
		ptr->prev = NULL;
		ptr->next = NULL;
	}
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
	if(block == free_list[block->order])
	{
		if((free_list[block->order] = block->next_free))
			free_list[block->order]->prev_free = NULL;
	}
	if(block->prev_free)
		block->prev_free->next_free = block->next_free;
	if(block->next_free)
		block->next_free->prev_free = block->prev_free;
	if(block->prev)
		block->prev->next = block->next;
	if(block->next)
		block->next->prev = block->prev;
	avl_tree_remove(&free_tree, &block->node);
}

/*
 * Splits the given block until a block of the required order is created and
 * returns it.
 * The input block will be unlinked and the new blocks created will be inserted
 * into the free list and free tree except the returned block.
 */
static buddy_free_block_t *split_block(buddy_free_block_t *block,
	const block_order_t order)
{
	debug_check_block(block);
	debug_check_order(order);
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
	ptr = split_block(free_list[i], order);
	spin_unlock(&spinlock);
	debug_check_block(ptr);
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
 * Allocates a block of memory using the buddy allocator in the specified range.
 */
ATTR_HOT
ATTR_MALLOC
void *buddy_alloc_inrange(const block_order_t order, void *begin, void *end)
{
	avl_tree_t *n;
	buddy_free_block_t *b;
	void *ptr;

	errno = 0;
	if(order > BUDDY_MAX_ORDER)
		return NULL;
	debug_assert(end >= begin, "buddy_alloc_inrange: invalid range");
	begin = ALIGN(begin, PAGE_SIZE);
	end = DOWN_ALIGN(end, PAGE_SIZE);
	// TODO Restrain `begin` and `end` to buddy allocator range
	spin_lock(&spinlock);
	if(!(n = get_nearest_free_block(begin)))
	{
		spin_unlock(&spinlock);
		errno = ENOMEM;
		return NULL;
	}
	b = CONTAINER_OF(n, buddy_free_block_t, node);
	// TODO Some previous blocks might be in the range?
	while(b && (void *) b < end && b->order < order)
		b = b->next;
	if(!b || b->order < order)
	{
		spin_unlock(&spinlock);
		errno = ENOMEM;
		return NULL;
	}
	ptr = split_block(b, order);
	spin_unlock(&spinlock);
	debug_check_block(ptr);
	return ptr;
}

/*
 * Uses `buddy_alloc_inrange` and applies `bzero` on the allocated block.
 */
ATTR_MALLOC
void *buddy_alloc_zero_inrange(block_order_t order, void *begin, void *end)
{
	void *ptr;

	if((ptr = buddy_alloc_inrange(order, begin, end)))
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
