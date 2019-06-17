#include "slab.h"

static cache_t *caches;
static cache_t *caches_cache;

__attribute__((hot))
static void cache_init(cache_t *cache, void *mem)
{
	const size_t size = caches_cache->slabs * PAGE_SIZE;
	slab_t *slab = cache->slabs_free;
	slab_t *prev_slab = NULL;

	while((void *) slab < (void *) mem + size)
	{
		if(prev_slab) prev_slab->next = slab;

		object_t *obj = slab->free_list = (void *) slab + sizeof(slab_t);
		object_t *prev_obj = NULL;

		while((void *) obj < (void *) slab + PAGE_SIZE)
		{
			if(prev_obj) prev_obj->next_free = obj;

			if(cache->ctor)
				cache->ctor(OBJ_CONTENT(obj), cache->objsize);

			prev_obj = obj;
			obj = OBJ_NEXT(obj, cache->objsize);
		}

		prev_slab = slab;
		slab = ALIGN_UP(slab, PAGE_SIZE); // TODO Adapt to the number of pages required for a single object
	}
}

__attribute__((cold))
void slab_init(void)
{
	caches = caches_cache = buddy_alloc_zero(CACHES_CACHE_ORDER);
	if(!caches_cache) PANIC("Cannot allocate cache for slab allocator!", 0);

	caches_cache->name = CACHES_CACHE_NAME;
	caches_cache->slabs = POW2(CACHES_CACHE_ORDER);
	caches_cache->objsize = sizeof(cache_t);
	const size_t size = caches_cache->slabs * PAGE_SIZE;
	caches_cache->objects_count = (size - sizeof(cache_t))
		/ caches_cache->objsize;
	caches_cache->slabs_free = (void *) caches_cache + sizeof(cache_t);

	cache_init(caches_cache, caches_cache);

	// TODO Create a cache for slabs?
}

__attribute__((hot))
cache_t *cache_getall(void)
{
	return caches;
}

__attribute__((hot))
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

__attribute__((hot))
static inline size_t required_size(const size_t objsize,
	const size_t objects_count)
{
	return sizeof(cache_t) + OBJ_TOTAL_SIZE(objsize) * objects_count;
}

// TODO Spinlock
__attribute__((hot))
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

	cache_t *cache;
	if(!(cache = cache_alloc(caches_cache))) return NULL;

	void *mem;
	if(!(mem = buddy_alloc_zero(order)))
	{
		cache_free(caches_cache, cache);
		return NULL;
	}

	cache->name = name;
	cache->slabs = pages;
	cache->objsize = objsize;
	cache->objects_count = objects_count;
	cache->slabs_free = mem;
	cache->ctor = ctor;
	cache->dtor = dtor;

	cache_init(cache, mem);

	cache_t *c = caches;
	while(c->next) c = c->next;
	c->next = cache;

	return cache;
}

// TODO Spinlock
__attribute__((hot))
void cache_shrink(cache_t *cache)
{
	// TODO
	(void) cache;
}

// TODO Spinlock
__attribute__((hot))
void *cache_alloc(cache_t *cache)
{
	if(!cache) return NULL;
	//spin_lock(&(cache->spinlock));

	object_t *obj = NULL;

	if(cache->slabs_partial && cache->slabs_partial->free_list)
	{
		obj = cache->slabs_partial->free_list;
		cache->slabs_partial->free_list = obj->next_free;
	}
	else if(cache->slabs_free && cache->slabs_free->free_list)
	{
		obj = cache->slabs_free->free_list;
		cache->slabs_free->free_list = obj->next_free;
	}
	else
	{
		// TODO Alloc new slab(s)?
		// TODO spin_unlock(&(cache->spinlock));
		return NULL;
	}

	obj->state |= OBJ_USED;
	// TODO Move slab (free -> partial or partial -> full)

	//spin_unlock(&(cache->spinlock));
	return OBJ_CONTENT(obj);
}

// TODO Spinlock
__attribute__((hot))
void cache_free(cache_t *cache, void *obj)
{
	// TODO
	(void) cache;
	(void) obj;
}

// TODO Spinlock
__attribute__((hot))
void cache_destroy(cache_t *cache)
{
	// TODO
	(void) cache;
}
