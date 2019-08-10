#include <memory/slab/slab.h>
#include <libc/errno.h>

// TODO Set errnos

static cache_t *caches;
static cache_t *caches_cache;

__attribute__((cold))
void slab_init(void)
{
	if(!(caches_cache = kmalloc_zero(sizeof(cache_t), 0)))
		PANIC("Failed to initialize slab allocator!", 0);
	caches_cache->name = CACHES_CACHE_NAME;
	caches_cache->objsize = sizeof(cache_t);
	caches_cache->objects_count = 32;
	// TODO cache_init(caches_cache);
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

	c = caches; 
	while(c)
	{
		if(strcmp(c->name, name) == 0)
			return c;
		c = c->next;
	}
	return NULL;
}

// TODO
__attribute__((hot))
cache_t *cache_create(const char *name, size_t objsize, size_t objects_count,
	void (*ctor)(void *, size_t), void (*dtor)(void *, size_t))
{
	// TODO
	(void) name;
	(void) objsize;
	(void) objects_count;
	(void) ctor;
	(void) dtor;
	return NULL;
}

__attribute__((hot))
void cache_shrink(cache_t *cache)
{
	if(!cache)
		return;
	lock(&cache->spinlock);
	// TODO
	(void) cache;
	unlock(&cache->spinlock);
}

__attribute__((hot))
void *cache_alloc(cache_t *cache)
{
	// TODO
	(void) cache;
	return NULL;
}

__attribute__((hot))
void cache_free(cache_t *cache, void *obj)
{
	if(!cache || !obj)
		return;
	lock(&cache->spinlock);
	// TODO
	(void) cache;
	(void) obj;
	unlock(&cache->spinlock);
}

__attribute__((hot))
void cache_destroy(cache_t *cache)
{
	if(!cache)
		return;
	lock(&cache->spinlock);
	// TODO
	(void) cache;
	unlock(&cache->spinlock);
}
