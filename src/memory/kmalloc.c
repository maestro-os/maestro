#include "memory.h"
#include "kalloc_internal.h"

malloc_chunk_t *new_chunks()
{
	// TODO void *ptr = paging_alloc(NULL, 1, 0);
	//bzero(ptr, PAGE_SIZE);

	//return ptr;
	return NULL;
}

void *kmalloc(const size_t size)
{
	if(size == 0) return NULL;

	static malloc_chunk_t *chunks_begin = NULL;
	if(!chunks_begin && !(chunks_begin = new_chunks())) return NULL;

	// TODO

	// TODO If small alloc, check if a page is available (how to store pointers to pages?)
	// TODO If no page is available, find a free page (using paging manager)
	// TODO Mark the page as used in paging table
	// TODO Return the pointer

	return NULL;
}
