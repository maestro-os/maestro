#include "memory.h"
#include "memory_internal.h"

#define BLOCKS_COUNT	((size_t) memory_end / BLOCK_SIZE)

static size_t max_order;
static buddy_t *buddies;
static uint16_t *pages_state;

static size_t get_max_order()
{
	size_t i = BLOCK_SIZE;
	size_t order = 0;

	while(i < (size_t) memory_end)
	{
		i *= 2;
		++order;
	}

	return order;
}

static size_t upper_division(const size_t n0, const size_t n1)
{
	return (n0 % n1 == 0 ? n0 / n1 : n0 / n1 + 1);
}

void buddy_init()
{
	max_order = get_max_order();

	const size_t buddies_size = BLOCKS_COUNT * sizeof(buddy_t);
	const size_t pages_state_size = BLOCKS_COUNT * PAGES_PER_BLOCK
		* sizeof(uint16_t);
	const size_t metadata_size = buddies_size + pages_state_size;
	const size_t metadata_blocks = upper_division(metadata_size, BLOCK_SIZE);
	// TODO Set reserved from 0x0 to HEAP_BEGIN
	(void) metadata_blocks;

	buddies = HEAP_BEGIN;
	pages_state = (void *) buddies + buddies_size;
}

buddy_t *buddy_get(void *ptr)
{
	// TODO
	(void) ptr;
	return NULL;
}

buddy_t *buddy_alloc(const size_t blocks)
{
	// TODO
	(void) blocks;
	return NULL;
}

void buddy_free(buddy_t *buddy)
{
	// TODO
	(void) buddy;
}
