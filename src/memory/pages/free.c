#include <memory/pages/pages.h>
#include <memory/pages/pages_internal.h>
#include <kernel.h>

/*
 * Frees the given number of pages in the given memory region.
 */
void pages_free(void *ptr, const size_t pages)
{
	pages_block_t *b;

	if(!sanity_check(ptr) || pages == 0)
		return;
	if(!(b = get_used_block(ptr)) || b->pages != pages)
		PANIC("Pages block being freed was not allocated", 0);
	free_block(b);
}
