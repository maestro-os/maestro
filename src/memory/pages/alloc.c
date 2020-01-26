#include <memory/pages/pages.h>
#include <memory/pages/pages_internal.h>

/*
 * Allocates a memory region of `n` pages and returns a pointer to the
 * beginning.
 *
 * The given memory region shall be freed using `pages_free`.
 */
ATTR_MALLOC
void *pages_alloc(const size_t n)
{
	pages_block_t *b;

	if(!(b = get_available_block(n)))
		b = alloc_block(n);
	if(!b)
		return NULL;
	split_block(b, n);
	return b->ptr;
}

/*
 * Calls `pages_alloc` and initializes the memory region to zero.
 */
ATTR_MALLOC
void *pages_alloc_zero(const size_t n)
{
	void *ptr;

	if((ptr = pages_alloc(n)))
		bzero(ptr, n * PAGE_SIZE);
	return ptr;
}
