#include <memory/pages/pages_internal.h>
#include <util/util.h>

ATTR_BSS
static pages_block_t *free_buckets[BUCKETS_COUNT];
ATTR_BSS
static pages_block_t *used_map[HASH_MAP_SIZE];

/*
 * Returns the free bucket index for a block of `n` pages.
 */
static size_t get_bucket_index(const size_t n)
{
	size_t i = 0;

	while(i - 1 < BUCKETS_COUNT && n > ((size_t) 1 << i))
		++i;
	return i;
}

/*
 * Links the specified block to either a bucket or the hash map according to
 * whether it is used or not.
 */
static void link_block(pages_block_t *b)
{
	pages_block_t **bucket;

	if(b->used)
		bucket = used_map + ((uintptr_t) b->ptr % HASH_MAP_SIZE);
	else
		bucket = free_buckets + get_bucket_index(b->pages);
	if((b->next = *bucket))
		b->next->prev = b;
	b->prev = NULL;
	*bucket = b;
}

/*
 * Unlinks the specified block from the specified bucket.
 */
static void unlink_block(pages_block_t **bucket, pages_block_t *b)
{
	if(b->prev)
		b->prev->next = b->next;
	if(b->next)
		b->next->prev = b->prev;
	// TODO Set b->prev and b->next to NULL?
	if(*bucket == b)
		*bucket = (*bucket)->next;
}

/*
 * Returns an unused block of pages that is at least `n` pages long.
 * If a free block is found, it will be unlinked from its bucket.
 */
pages_block_t *get_available_block(const size_t n)
{
	size_t i = 0;
	pages_block_t **bucket, *b;

	while(i < BUCKETS_COUNT - 1 && n > ((size_t) 1 << i))
		++i;
	while(i < BUCKETS_COUNT - 1 && !free_buckets[i])
		++i;
	bucket = free_buckets + i;
	if(!(b = *bucket))
		return NULL;
	while(b && b->pages < n)
		b = b->next;
	unlink_block(bucket, b);
	b->used = 1;
	link_block(b);
	return b;
}

/*
 * Allocates a block of memory using the buddy allocator.
 * The block of memory shall be at least `n` pages large and shall be marked as
 * used.
 */
pages_block_t *alloc_block(const size_t n)
{
	size_t pages;
	void *ptr;
	pages_block_t *b;

	pages = buddy_get_order(n);
	if(!(ptr = buddy_alloc(pages)))
		return NULL;
	if(!(b = pages_block_alloc(ptr, pages)))
	{
		buddy_free(ptr);
		return NULL;
	}
	b->used = 1;
	link_block(b);
	return b;
}

/*
 * Shrinks the given block to `n` pages. A new block might be created that
 * contains the remaining pages.
 * The given block must be marked as used.
 */
void split_block(pages_block_t *b, const size_t n)
{
	pages_block_t *new;

	if(b->pages <= n)
		return;
	if(!(new = pages_block_alloc(b->ptr + n * PAGE_SIZE, b->pages - n)))
		return;
	b->pages = n;
	if((new->buddy_next = b->buddy_next))
		new->buddy_next->buddy_prev = new;
	if((new->buddy_prev = b))
		new->buddy_prev->buddy_next = new;
	link_block(new);
}
