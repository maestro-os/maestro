#include <kernel.h>
#include <memory/slab/slab.h>
#include <libc/errno.h>

static cache_t *caches;
static cache_t *caches_cache;

__attribute__((hot))
static void calc_pages_per_slab(cache_t *cache)
{
	cache->pages_per_slab = CEIL_DIVISION(sizeof(slab_t)
		+ CEIL_DIVISION(cache->objcount, 8)
			+ (cache->objsize * cache->objcount), PAGE_SIZE);
}

__attribute__((cold))
void slab_init(void)
{
	if(!(caches_cache = kmalloc_zero(sizeof(cache_t), 0)))
		PANIC("Failed to initialize slab allocator!", 0);
	caches_cache->name = CACHES_CACHE_NAME;
	caches_cache->objsize = sizeof(cache_t);
	caches_cache->objcount = 32;
	calc_pages_per_slab(caches_cache);
	caches_cache->ctor = bzero;
	caches = caches_cache;
}

__attribute__((hot))
cache_t *cache_getall(void)
{
	return caches;
}

__attribute__((hot))
cache_t *cache_get(const char *name)
{
	cache_t *c;

	if(!name)
		return NULL;
	c = caches; 
	while(c)
	{
		if(strcmp(c->name, name) == 0)
			return c;
		c = c->next;
	}
	return NULL;
}

__attribute__((hot))
cache_t *cache_create(const char *name, size_t objsize, size_t objcount,
	void (*ctor)(void *, size_t), void (*dtor)(void *, size_t))
{
	cache_t *cache;

	if(!name || objsize == 0 || objcount == 0
		|| !(cache = cache_alloc(caches_cache)))
		return NULL;
	cache->name = name;
	cache->objsize = objsize;
	cache->objcount = objcount;
	calc_pages_per_slab(cache);
	cache->ctor = ctor;
	cache->dtor = dtor;
	cache->next = caches;
	caches = cache;
	return cache;
}

__attribute__((hot))
static void unlink_slab(slab_t *slab)
{
	if(!slab)
		return;
	if(slab->next)
		slab->next->prev = slab->prev;
	if(slab->prev)
		slab->prev->next = slab->next;
}

__attribute__((hot))
static slab_t *alloc_slab(cache_t *cache)
{
	slab_t *slab;

	if(!(slab = pages_alloc_zero(cache->pages_per_slab)))
		return NULL;
	slab->available = cache->objcount;
	if((slab->next = cache->slabs_free))
		slab->next->prev = slab;
	cache->slabs_free = slab;
	return slab;
}

__attribute__((hot))
void *cache_alloc(cache_t *cache)
{
	slab_t *slab;
	size_t i;
	void *ptr;

	if(!cache)
		return NULL;
	if(!(slab = cache->slabs_partial) && !(slab = cache->slabs_free))
	{
		if(!(slab = alloc_slab(cache)))
			return NULL;
	}
	i = bitfield_first_clear(SLAB_BITMAP(slab), cache->objcount);
	bitfield_set(SLAB_BITMAP(slab), i);
	--slab->available;
	if(slab->available == 0)
	{
		unlink_slab(slab);
		slab->next = cache->slabs_full;
		slab->prev = NULL;
		cache->slabs_full = slab;
	}
	else if(slab->available < cache->objcount)
	{
		unlink_slab(slab);
		slab->next = cache->slabs_partial;
		slab->prev = NULL;
		cache->slabs_partial = slab;
	}
	ptr = SLAB_OBJ(cache, slab, i);
	if(cache->ctor)
		cache->ctor(ptr, cache->objsize);
	return ptr;
}

__attribute__((hot))
void cache_shrink(cache_t *cache)
{
	if(!cache)
		return;
	spin_lock(&cache->spinlock);
	// TODO
	(void) cache;
	spin_unlock(&cache->spinlock);
}

__attribute__((hot))
void cache_free(cache_t *cache, void *obj)
{
	if(!cache || !obj)
		return;
	spin_lock(&cache->spinlock);
	// TODO
	(void) cache;
	(void) obj;
	spin_unlock(&cache->spinlock);
}

__attribute__((hot))
void cache_destroy(cache_t *cache)
{
	if(!cache)
		return;
	spin_lock(&cache->spinlock);
	// TODO
	(void) cache;
	spin_unlock(&cache->spinlock);
}
