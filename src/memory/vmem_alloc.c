#include <memory/memory.h>
#include <libc/errno.h>

// TODO Stack allocation
// TODO Find a way to organize pages

__attribute__((hot))
void *vmem_alloc_pages(vmem_t vmem, const size_t pages)
{
	void *ptr;

	if(!vmem || !(ptr = pages_alloc_zero(pages)))
		return NULL;
	vmem_identity_range(vmem, ptr, ptr + (pages * PAGE_SIZE),
		PAGING_PAGE_USER | PAGING_PAGE_WRITE);
	if(errno)
	{
		pages_free(ptr, pages);
		return NULL;
	}
	return ptr;
}

__attribute__((hot))
void vmem_free_pages(vmem_t vmem, const size_t pages, const int mem_free)
{
	if(!vmem)
		return;
	// TODO
	(void) pages;
	(void) mem_free;
}
