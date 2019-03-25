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

void *krealloc(void *ptr, const size_t size)
{
	if(!ptr || size == 0) return NULL;

	// TODO
	return NULL;
}

void kfree(void *ptr)
{
	if(!ptr) return;

	// TODO Clean memory?
	// TODO Mark memory as free
	(void) ptr;
}

void *mm_alloc(const pid_t pid, void *hint, const size_t length,
	const uint16_t flags)
{
	// TODO Use kmalloc
	(void) pid;
	(void) hint;
	(void) length;
	(void) flags;

	return NULL;
}

void mm_free(void *ptr)
{
	// TODO Use kfree
	(void) ptr;
}

void mm_free_pid(const pid_t pid)
{
	// TODO Use kfree
	(void) pid;
}
