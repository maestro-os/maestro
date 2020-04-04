#ifndef PAGES_INTERNAL_H
# define PAGES_INTERNAL_H

# include <memory/memory.h>
# include <util/util.h>

# define BUCKETS_COUNT	16
# define HASH_MAP_SIZE	1024

/*
 * Describes a portion of a block of memory that has been allocated by the
 * buddy allocator.
 */
typedef struct pages_block
{
	/*
	 * Double-linked list that stores blocks according to:
	 * - Their size if free
	 * - Their pointer if used (using a hash map)
	 */
	list_head_t blocks_node;
	/* Double-linked list of blocks that are stored on the same buddy */
	list_head_t buddies_node;

	/* Pointer to the beginning of the memory region */
	void *ptr;
	/* Size of the memory region in pages */
	size_t pages;

	/* Tells whether the memory block is used or not */
	int used;
} pages_block_t;

pages_block_t *get_available_block(size_t n);
pages_block_t *alloc_block(size_t n);
void split_block(pages_block_t *b, size_t n);
pages_block_t *get_used_block(void *ptr);
void free_block(pages_block_t *b);

#endif
