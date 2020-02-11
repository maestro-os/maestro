#include <memory/pages/pages_internal.h>

/*
 * Pointer to the first non-full `block_cache` structure.
 * The list of `block_cache` structures is sorted in increasing order.
 */
static blocks_cache_t *blocks_caches = NULL;
/*
 * Pointer to the last `blocks_cache` structure
 */
static blocks_cache_t *last_blocks_cache = NULL;

/*
 * Updates the `first_available` field for the given block_cache_t structure.
 */
static void update_first_available(blocks_cache_t *block)
{
	pages_block_t *b;

	b = block->first_available;
	do
	{
		if(!b->ptr)
		{
			block->first_available = b;
			return;
		}
		b = (void *) b + sizeof(pages_block_t);
		if((void *) b >= (void *) block + PAGE_SIZE)
			b = (void *) block + sizeof(blocks_cache_t);
	}
	while(b != block->first_available);
	block->first_available = NULL;
}

/*
 * Updates variable `blocks_caches`
 */
static void update_cursor(void)
{
	if(!blocks_caches)
		return;
	while(blocks_caches->prev && blocks_caches->prev->available > 0)
		blocks_caches = blocks_caches->prev;
	while(blocks_caches->next && blocks_caches->available == 0)
		blocks_caches = blocks_caches->next;
}

/*
 * Allocates a structure `pages_block_t` and returns a pointer to it.
 */
pages_block_t *pages_block_alloc(void *ptr, const size_t pages)
{
	blocks_cache_t *b;
	pages_block_t *block;

	if(!(b = blocks_caches) || b->available == 0)
	{
		if(!(b = buddy_alloc_zero(0)))
			return NULL;
		if((b->prev = last_blocks_cache))
			b->prev->next = b;
		if(!blocks_caches)
			blocks_caches = b;
		last_blocks_cache = b;
		b->available = BLOCKS_INFO_CAPACITY;
		b->first_available = (void *) b + sizeof(blocks_cache_t);
	}
	--b->available;
	if(b == blocks_caches)
		update_cursor();
	block = b->first_available;
	bzero(block, sizeof(pages_block_t));
	block->ptr = ptr;
	block->pages = pages;
	update_first_available(b);
	return block;
}

/*
 * Frees the given structure `pages_block_t`.
 */
void pages_block_free(pages_block_t *block)
{
	blocks_cache_t *b;

	if(!block)
		return;
	block->ptr = NULL;
	b = DOWN_ALIGN(block, PAGE_SIZE);
	if(++b->available >= BLOCKS_INFO_CAPACITY)
	{
		if(b == blocks_caches)
			blocks_caches = (blocks_caches->prev
				? blocks_caches->prev : blocks_caches->next);
		if(b == last_blocks_cache)
			last_blocks_cache = last_blocks_cache->prev;
		if(b->next)
			b->next->prev = b->prev;
		if(b->prev)
			b->prev->next = b->next;
		buddy_free(b, 0);
	}
	update_cursor();
}
