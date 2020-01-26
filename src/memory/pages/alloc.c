#include <memory/pages/pages.h>

ATTR_MALLOC
void *pages_alloc(const size_t n)
{
	// TODO
	(void) n;
	return NULL;
}

ATTR_MALLOC
void *pages_alloc_zero(const size_t n)
{
	void *ptr;

	if((ptr = pages_alloc(n)))
		bzero(ptr, n * PAGE_SIZE);
	return ptr;
}
