#include "slab.h"

static cache_t *caches;
static cache_t *caches_cache;

void slab_init()
{
	caches = NULL;
	caches_cache = NULL;
	// TODO Create a cache for caches
}

cache_t *cache_getall()
{
	return caches;
}

cache_t *cache_get(const char *name)
{
	cache_t *c = caches;

	while(c)
	{
		if(strcmp(c->name, name) == 0)
			return c;
		c = c->next;
	}

	return NULL;
}

static inline size_t required_size(const size_t objsize,
	const size_t objects_count)
{
	return sizeof(cache_t) + (objsize + sizeof(object_t)) * objects_count;
}

cache_t *cache_create(const char *name, const size_t objsize,
	const size_t objects_count, void (*ctor)(void *, size_t),
		void (*dtor)(void *, size_t))
{
	size_t size = required_size(objsize, objects_count);
	size_t pages = UPPER_DIVISION(size, PAGE_SIZE);

	size_t order = 1;
	while((size_t) POW2(order) < pages) order <<= 1;
	pages = POW2(order);
	size = pages * PAGE_SIZE;
	// TODO Increase objects_count up to cache capacity?

	void *mem;
	if(!(mem = buddy_alloc(order))) return NULL;

	cache_t *cache;
	if(!(cache = cache_alloc(caches_cache)))
	{
		buddy_free(mem);
		return NULL;
	}

	cache->name = name;
	cache->objsize = objsize;
	cache->objects_count = objects_count;
	cache->slabs = pages;

	// TODO Fill slabs_*

	cache->ctor = ctor;
	cache->dtor = dtor;

	if(cache->ctor)
	{
		object_t *obj = cache->slabs_free.free_objs;

		while(obj)
		{
			cache->ctor(obj + sizeof(object_t), cache->objsize);
			obj = obj->next;
		}
	}

	return cache;
}

void cache_shrink(cache_t *cache)
{
	// TODO
	(void) cache;
}

void *cache_alloc(cache_t *cache)
{
	if(!cache) return NULL;
	spin_lock(&cache->spinlock);

	object_t *obj;

	if((obj = cache->slabs_partial.free_objs))
		cache->slabs_partial.free_objs = obj->next;
	else if((obj = cache->slabs_free.free_objs))
		cache->slabs_free.free_objs = obj->next;
	else
	{
		spin_unlock(&cache->spinlock);
		return NULL;
	}

	spin_unlock(&cache->spinlock);
	return obj;
}

void cache_free(cache_t *cache, void *obj)
{
	// TODO
	(void) cache;
	(void) obj;
}

void cache_destroy(cache_t *cache)
{
	// TODO
	(void) cache;
}
