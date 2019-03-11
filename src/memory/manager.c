#include "memory.h"

#include "../libc/errno.h"
// TODO #include "../libc/math.h"

static inline size_t get_max_level()
{
	// TODO return log2(memory_end / PAGE_SIZE);
	return 0;
}

void mm_init()
{
	memory_block_t *root = HEAP_BEGIN;
	root->left = NULL;
	root->right = NULL;

	size_t level = get_max_level();

	// TODO
	(void) level;
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
