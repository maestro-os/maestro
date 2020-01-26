#include <memory/pages/pages_internal.h>

/*
 * Pointer to the first non-full `block_info` structure.
 * The list of `block_info` structures is sorted in increasing order.
 */
static blocks_info_t *blocks_infos = NULL;
static blocks_info_t *last_blocks_info = NULL;

/*
 * Updates the `first_available` field for the given block_info_t structure.
 */
static void update_first_available(blocks_info_t *block)
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
			b = (void *) block + sizeof(blocks_info_t);
	}
	while(b != block->first_available);
	block->first_available = NULL;
}

/*
 * Updates variable `blocks_infos`
 */
static void update_cursor(void)
{
	if(!blocks_infos)
		return;
	while(blocks_infos->prev && blocks_infos->prev->available > 0)
		blocks_infos = blocks_infos->prev;
	while(blocks_infos->next && blocks_infos->available == 0)
		blocks_infos = blocks_infos->next;
}

/*
 * Allocates a structure `pages_block_t` and returns a pointer to it.
 */
pages_block_t *pages_block_alloc(void *ptr, const size_t pages)
{
	blocks_info_t *b;
	pages_block_t *block;

	if(!(b = blocks_infos) || b->available == 0)
	{
		if(!(b = buddy_alloc_zero(0)))
			return NULL;
		if((b->prev = last_blocks_info))
			b->prev->next = b;
		if(!blocks_infos)
			blocks_infos = b;
		last_blocks_info = b;
		b->available = BLOCKS_INFO_CAPACITY;
		b->first_available = (void *) b + sizeof(blocks_info_t);
	}
	--b->available;
	if(b == blocks_infos)
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
	blocks_info_t *b;

	if(!block)
		return;
	block->ptr = NULL;
	b = DOWN_ALIGN(block, PAGE_SIZE);
	if(++b->available >= BLOCKS_INFO_CAPACITY)
	{
		if(b == blocks_infos)
			blocks_infos = blocks_infos->prev;
		if(b == last_blocks_info)
			last_blocks_info = last_blocks_info->prev;
		if(b->next)
			b->next->prev = b->prev;
		if(b->prev)
			b->prev->next = b->next;
		buddy_free(b);
	}
	update_cursor();
}
