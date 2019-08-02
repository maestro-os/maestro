#include <memory/kmalloc_internal.h>
#include <libc/errno.h>

__attribute__((cold))
void kmalloc_init(void)
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

__attribute__((hot))
static cache_t *get_cache(const size_t size)
{
	size_t i;

	for(i = 0; i < KMALLOC_CACHES_COUNT; ++i)
	{
		if(!kmalloc_caches[i])
			continue;
		if(kmalloc_caches[i]->objsize >= size)
			return kmalloc_caches[i];
	}
	return NULL;
}

__attribute__((hot))
void *kmalloc(const size_t size)
{
	cache_t *cache;
	void *ptr;

	if(size == 0)
		return NULL;
	errno = 0;
	if((cache = get_cache(size)))
		ptr = cache_alloc(cache);
	else
	{
		// TODO Use unused space for further allocations
		ptr = buddy_alloc(buddy_get_order(size));
	}
	if(!ptr)
		errno = ENOMEM;
	return ptr;
}

__attribute__((hot))
void *kmalloc_zero(const size_t size)
{
	void *ptr;

	if((ptr = kmalloc(size)))
		bzero(ptr, size);
	return ptr;
}
