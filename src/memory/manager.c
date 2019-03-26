#include "memory.h"

static uint32_t default_directory[1024] __attribute__((aligned(4096)));

void mm_init()
{
	paging_create_directory(default_directory);

	// TODO Identity

	// TODO paging_enable(default_directory);
}

void *mm_map(void *hint, const size_t pages, const uint8_t flags)
{
	// TODO
	(void) hint;
	(void) pages;
	(void) flags;

	return NULL;
}

void mm_munmap(void *ptr)
{
	// TODO
	(void) ptr;
}
