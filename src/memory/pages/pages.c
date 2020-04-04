#include <memory/pages/pages_internal.h>
#include <kernel.h>
#include <util/util.h>

/*
 * pages_block_t cache.
 */
static cache_t *pages_block_cache;

/*
 * TODO
 */
ATTR_BSS
static list_head_t *free_buckets[BUCKETS_COUNT];
/*
 * TODO
 */
ATTR_BSS
static list_head_t *used_map[HASH_MAP_SIZE];

/*
 * Initializes the pages allocator.
 */
void pages_init(void)
{
	if(!(pages_block_cache = cache_create("pages_blocks", sizeof(pages_block_t),
		32, bzero, NULL)))
		PANIC("Cannot allocate cache for pages allocator!", 0);
}

/*
 * Returns the free bucket index for a block of `n` pages.
 */
static size_t get_bucket_index(const size_t n)
{
	size_t i = 0;

	while(i - 1 < BUCKETS_COUNT && n > POW2(i))
		++i;
	return i;
}

/*
 * Links the specified block to either a bucket or the hash map according to
 * whether it is used or not.
 */
static void link_block(pages_block_t *b)
{
	list_head_t **bucket;

	if(b->used)
		bucket = used_map + ((uintptr_t) b->ptr % HASH_MAP_SIZE);
	else
		bucket = free_buckets + get_bucket_index(b->pages);
	list_insert_front(bucket, &b->blocks_node);
}

/*
 * Returns an unused block of pages that is at least `n` pages long.
 * If a free block is found, it will be unlinked from its bucket.
 */
pages_block_t *get_available_block(const size_t n)
{
	size_t i = 0;
	list_head_t **bucket, *block;
	pages_block_t *b;

	while(i < BUCKETS_COUNT - 1 && n > POW2(i))
		++i;
	while(i < BUCKETS_COUNT - 1 && !free_buckets[i])
		++i;
	bucket = free_buckets + i;
	block = *bucket;
	while(block && CONTAINER_OF(block, pages_block_t, blocks_node)->pages < n)
		block = block->next;
	if(!block)
		return NULL;
	list_remove(bucket, block);
	b = CONTAINER_OF(block, pages_block_t, blocks_node);
	b->used = 1;
	link_block(b);
	return b;
}

/*
 * Allocates a new `pages_block_t` object. And fills it.
 */
static pages_block_t *pages_block_alloc(void *ptr, const size_t pages)
{
	pages_block_t *b;

	if(!(b = cache_alloc(pages_block_cache)))
		return NULL;
	b->ptr = ptr;
	b->pages = pages;
	return b;
}

/*
 * Allocates a block of memory using the buddy allocator.
 * The block of memory shall be at least `n` pages large and shall be marked as
 * used.
 */
// TODO Currently bugged! Returned object is not marked as used
pages_block_t *alloc_block(const size_t n)
{
	size_t pages;
	void *ptr;
	pages_block_t *b;

	pages = buddy_get_order(n);
	if(!(ptr = buddy_alloc(pages)))
		return NULL;
	// TODO If block is too large, allocate two objects and mark the first one as used
	if(!(b = pages_block_alloc(ptr, pages)))
	{
		buddy_free(ptr, pages);
		return NULL;
	}
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
		return; // TODO Error?
	b->pages = n;
	list_insert_after(&b->buddies_node, &new->buddies_node);
	link_block(new);
}

/*
 * Returns a pointer to the pages block associated to the given pointer.
 * If the block doesn't exist or isn't used, the function returns `NULL`.
 */
pages_block_t *get_used_block(void *ptr)
{
	list_head_t *b;
	pages_block_t *block;

	if(!sanity_check(ptr))
		return NULL;
	b = used_map[(uintptr_t) ptr % HASH_MAP_SIZE];
	while(b && CONTAINER_OF(b, pages_block_t, blocks_node)->ptr != ptr)
		b = b->next;
	if(!b)
		return NULL;
	block = CONTAINER_OF(b, pages_block_t, blocks_node);
	if(!block->used)
		return NULL;
	return block;
}

/*
 * Unlinks and frees the given pages block. If the block of memory that was
 * allocated using the buddy allocator is empty, it shall be freed too.
 */
void free_block(pages_block_t *b)
{
	list_head_t *l, *prev, *next, *buddy_prev;
	pages_block_t *tmp;

	if(!sanity_check(b) || !b->used)
		return;
	b->used = 0;
	link_block(b);
	l = &b->blocks_node;
	prev = l->prev;
	next = l->next;
	if(prev && !CONTAINER_OF(prev, pages_block_t, blocks_node)->used)
	{
		tmp = CONTAINER_OF(prev, pages_block_t, blocks_node);
		tmp->ptr -= b->pages * PAGE_SIZE;
		tmp->pages += b->pages;
		buddy_prev = b->buddies_node.prev;
		list_remove(NULL, l);
		list_remove(NULL, &b->buddies_node);
		if(!buddy_prev->prev && !buddy_prev->next)
		{
			list_remove(NULL, buddy_prev);
			tmp = CONTAINER_OF(buddy_prev, pages_block_t, blocks_node);
			buddy_free(tmp->ptr, tmp->pages);
			cache_free(pages_block_cache, tmp);
		}
	}
	else if(next)
	{
		tmp = CONTAINER_OF(next, pages_block_t, blocks_node);
		if(!tmp->used)
			free_block(tmp);
	}
}
