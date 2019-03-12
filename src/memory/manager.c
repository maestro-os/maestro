#include "memory.h"

void mm_init()
{
	// TODO
}

size_t mm_required_pages(const size_t length)
{
	const size_t pages = (length / PAGE_SIZE);
	return (length % PAGE_SIZE == 0 ? pages : pages + 1);
}

page_t *mm_alloc_pages(const pid_t pid, void *hint, const size_t count)
{
	// TODO
	(void) pid;
	(void) hint;
	(void) count;

	return NULL;
}

void mm_free(void *ptr)
{
	// TODO
	(void) ptr;
}
