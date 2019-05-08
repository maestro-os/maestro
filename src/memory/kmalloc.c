#include "kmalloc_internal.h"
#include "../libc/errno.h"

void kmalloc_init()
{
	kmalloc_caches[0] = cache_create("kmalloc8", 8, PAGE_SIZE / 8,
		NULL, bzero);
	kmalloc_caches[1] = cache_create("kmalloc16", 16, PAGE_SIZE / 16,
		NULL, bzero);
	kmalloc_caches[2] = cache_create("kmalloc32", 32, PAGE_SIZE / 32,
		NULL, bzero);
	kmalloc_caches[3] = cache_create("kmalloc64", 64, PAGE_SIZE / 64,
		NULL, bzero);
	kmalloc_caches[4] = cache_create("kmalloc128", 128, PAGE_SIZE / 128,
		NULL, bzero);
	kmalloc_caches[5] = cache_create("kmalloc256", 256, PAGE_SIZE / 256,
		NULL, bzero);
	kmalloc_caches[6] = cache_create("kmalloc512", 512, PAGE_SIZE / 512,
		NULL, bzero);
	// TODO More kmalloc caches?
}

void *kmalloc(const size_t size)
{
	if(size == 0) return NULL;
	errno = 0;

	// TODO
	return NULL;
}
