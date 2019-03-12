#include "syscall.h"

void *mmap(void *addr, size_t length, int prot, int flags,
	int fd, off_t offset)
{
	if(length == 0)
	{
		errno = EINVAL;
		return NULL;
	}

	// TODO
	(void) prot;
	(void) flags;

	// TODO pid
	const page_t *page = mm_alloc_pages(0, addr, mm_required_pages(length));

	if(!page)
	{
		errno = ENOMEM;
		return NULL;
	}

	// TODO
	(void) fd;
	(void) offset;

	// TODO return page;
	return NULL;
}
