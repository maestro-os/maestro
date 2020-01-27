#ifndef PAGES_INTERNAL_H
# define PAGES_INTERNAL_H

# include <memory/memory.h>
# include <util/util.h>

# define BUCKETS_COUNT	16
# define HASH_MAP_SIZE	1024

/*
 * Number of structure that one `blocks_cache_t` can contain
 */
# define BLOCKS_INFO_CAPACITY	((PAGE_SIZE - sizeof(blocks_cache_t))\
	/ sizeof(pages_block_t))

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
	struct pages_block *prev, *next;
	/* Double-linked list of blocks that are stored on the same buddy */
	struct pages_block *buddy_prev, *buddy_next;

	/* Pointer to the beginning of the memory region */
	void *ptr;
	/* Size of the memory region in pages */
	size_t pages;

	/* Tells whether the memory block is used or not */
	int used;
} pages_block_t;

/*
 * Represents a page of memory allocated to store pages_block_t structures.
 */
typedef struct blocks_cache
{
	/*
	 * Double-linked list of blocks_info sorted according
	 * to increasing `available`.
	 */
	struct blocks_cache *prev, *next;

	/*
	 * Number of available structures.
	 */
	size_t available;
	/*
	 * Pointer to the first unused structure.
	 */
	pages_block_t *first_available;
} blocks_cache_t;

pages_block_t *get_available_block(size_t n);
pages_block_t *alloc_block(size_t n);
void split_block(pages_block_t *b, size_t n);

pages_block_t *pages_block_alloc(void *ptr, size_t pages);
void pages_block_free(pages_block_t *b);

#endif
