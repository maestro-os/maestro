#include "memory.h"

#include "../libc/errno.h"
#include "../libc/math.h"

static inline size_t get_max_level()
{
	return log2(memory_end / PAGE_SIZE);
}

static inline size_t get_level_size(const size_t level)
{
	return pow(2, level);
}

void mm_init()
{
	memory_block_t *root = HEAP_BEGIN;
	root->left = NULL;
	root->right = NULL;

	size_t level = get_max_level();
	memory_block_t *prev = root;

	// TODO Do not recompute level size each time?
	while(get_level_size(level - 1) > KERNEL_MIN)
	{
		memory_block_t *block = prev + sizeof(memory_block_t);
		block->left = NULL;
		block->RIGHT = NULL;

		prev->left = block;
		prev = block;
	}

	// TODO
}

size_t mm_required_pages(const size_t length)
{
	const size_t pages = (length / PAGE_SIZE);
	return (length % PAGE_SIZE == 0 ? pages : pages + 1);
}

page_t *mm_find_free_pages(void *hint, const size_t count)
{
	// TODO
	(void) hint;
	(void) count;

	return NULL;
}

void mm_free(void *ptr)
{
	(void) ptr;
	// TODO
}
