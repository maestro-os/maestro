#ifndef BUDDY_INTERNAL_H
# define BUDDY_INTERNAL_H

# include <memory/buddy/buddy.h>
# include <util/util.h>

typedef struct buddy_free_block
{
	/* Double-linked list of free blocks of the same order. */
	struct buddy_free_block *prev_free, *next_free;
	/* Double-linked list of free blocks ordered by pointer. */
	struct buddy_free_block *prev, *next;
	/* The AVL tree node. */
	avl_tree_t node;

	/* The block's order. */
	block_order_t order;
} buddy_free_block_t;

#endif
