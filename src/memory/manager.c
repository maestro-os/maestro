#include "memory.h"

void mm_init()
{
	// TODO
}

void *kmalloc(const size_t size)
{
	if(size == 0) return NULL;

	// TODO If small alloc, check if a page is available (how to store pointers to pages?)
	// TODO If no page is available, find a free page (using paging manager)
	// TODO Mark the page as used in paging table
	// TODO Return the pointer

	return NULL;
}

void kfree(void *ptr)
{
	if(!ptr) return;

	// TODO Clean memory?
	// TODO Mark memory as free
	(void) ptr;
}

size_t mm_required_pages(const size_t length)
{
	const size_t pages = (length / PAGE_SIZE);
	return (length % PAGE_SIZE == 0 ? pages : pages + 1);
}
