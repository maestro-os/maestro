#include "slab.h"

static cache_t *caches;
static cache_t *caches_cache;

void slab_init()
{
	caches = caches_cache = buddy_alloc_zero(CACHES_CACHE_ORDER);
	if(!caches_cache) PANIC("Cannot allocate cache for slab allocator!");

	caches_cache->name = CACHES_CACHE_NAME;
	caches_cache->slabs = POW2(CACHES_CACHE_ORDER);
	caches_cache->objsize = sizeof(cache_t);
	const size_t size = caches_cache->slabs * PAGE_SIZE;
	caches_cache->objects_count = (size - sizeof(cache_t))
		/ caches_cache->objsize;

	void *ptr = (void *) caches_cache + sizeof(cache_t);
	slab_t *prev = NULL;

	while(ptr < (void *) caches_cache + size)
	{
		if(prev) prev->next = ptr;
		ptr = ALIGN_UP(ptr, PAGE_SIZE);
		prev = ptr;
	}

	// TODO Create a cache for slabs?
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
	return sizeof(cache_t) + OBJ_TOTAL_SIZE(objsize) * objects_count;
}

#include "../../libc/stdio.h"

cache_t *cache_create(const char *name, size_t objsize, size_t objects_count,
	void (*ctor)(void *, size_t), void (*dtor)(void *, size_t))
{
	size_t size = required_size(objsize, objects_count);
	size_t pages = UPPER_DIVISION(size, PAGE_SIZE);

	size_t order = 1;
	while((size_t) POW2(order) < pages) ++order;
	pages = POW2(order);
	size = pages * PAGE_SIZE;
	// TODO Increase objects_count up to cache capacity?

	void *mem;
	if(!(mem = buddy_alloc_zero(order))) return NULL;

	cache_t *cache;
	if(!(cache = cache_alloc(caches_cache)))
	{
		buddy_free(mem);
		return NULL;
	}

	cache->name = name;
	cache->slabs = pages;
	cache->objsize = objsize;
	cache->objects_count = objects_count;

	void *ptr = mem;
	slab_t *prev = NULL;

	while(ptr < (void *) mem + size)
	{
		if(prev) prev->next = ptr;
		ptr = ALIGN_UP(ptr, PAGE_SIZE); // TODO Adapt to the number of pages required for a single object
		prev = ptr;
	}

	cache->ctor = ctor;
	cache->dtor = dtor;

	cache_t *c = caches;
	while(c->next) c = c->next;
	c->next = cache;

	if(!cache->ctor) return cache;

	slab_t *slab = cache->slabs_free;

	while(slab)
	{
		object_t *obj = slab->free_objs;

		// TODO Infinite loop
		while(obj)
		{
			cache->ctor(OBJ_CONTENT(obj), cache->objsize);
			obj = OBJ_NEXT(obj, objsize);
		}

		slab = slab->next;
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
	spin_lock(&(cache->spinlock));

	object_t *obj;

	if(cache->slabs_partial && (obj = cache->slabs_partial->free_objs))
		cache->slabs_partial->free_objs = OBJ_NEXT(obj, cache->objsize);
	else if(cache->slabs_free && (obj = cache->slabs_free->free_objs))
		cache->slabs_free->free_objs = OBJ_NEXT(obj, cache->objsize);
	else
	{
		spin_unlock(&(cache->spinlock));
		return NULL;
	}

	obj->state |= OBJ_USED;
	// TODO Move slab (free -> partial or partial -> full)

	spin_unlock(&(cache->spinlock));
	return OBJ_CONTENT(obj);
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
