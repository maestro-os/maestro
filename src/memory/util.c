#include <memory/memory.h>

__attribute__((hot))
void *clone_page(void *ptr)
{
	void *new_page;

	ptr = (void *) ((uintptr_t) ptr & PAGING_ADDR_MASK);
	if((new_page = buddy_alloc(0)))
		memcpy(new_page, ptr, PAGE_SIZE);
	return new_page;
}
