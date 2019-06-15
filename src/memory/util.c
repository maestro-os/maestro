#include "memory.h"

__attribute__((hot))
void *clone_page(void *ptr)
{
	ptr = (void *) ((uintptr_t) ptr & PAGING_ADDR_MASK);

	void *new_page;
	if((new_page = buddy_alloc(0)))
		memcpy(new_page, ptr, PAGE_SIZE);

	return new_page;
}
