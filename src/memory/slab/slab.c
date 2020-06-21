#include <kernel.h>
#include <memory/slab/slab.h>
#include <libc/errno.h>

/*
 * This file contains the slab allocator. This allocator allows to reduce
 * fragmentation by having regions of memory reserved to a specific object of a
 * fixed size, allowing to pack them together.
 *
 * A cache is an object that represents an allocator for a specific object.
 * A slab is one or several pages allocated for storing objects.
 * An object is an allocation on the slab allocator.
 */

// TODO Sort slabs so the that the allocator allocates objects on the most filled slabs first
// TODO Avoid having objects on two different pages (if possible) to reduce cache swap

/*
 * The list of all caches.
 */
static cache_t *caches;
/*
 * The cache used to allocate caches.
 */
static cache_t caches_cache;

/*
 * Computes the number of pages required for a slab.
 */
ATTR_HOT
static void calc_slab_order(cache_t *cache)
{
	size_t pages_count;

	debug_assert(sanity_check(cache), "calc_pages_per_slab: invalid argument");
	pages_count = CEIL_DIVISION(sizeof(slab_t)
		+ CEIL_DIVISION(cache->objcount, 8)
			+ (cache->objsize * cache->objcount), PAGE_SIZE);
	cache->slab_order = buddy_get_order(pages_count);
	// TODO Adapt objects count
}

/*
 * Initializes the slab allocator.
 */
ATTR_COLD
void slab_init(void)
{
	caches_cache.name = CACHES_CACHE_NAME;
	caches_cache.objsize = sizeof(cache_t);
	caches_cache.objcount = 32;
	calc_slab_order(&caches_cache);
	caches_cache.ctor = bzero;
	caches = &caches_cache;
}

/*
 * Returns the list of all caches.
 */
ATTR_HOT
cache_t *cache_getall(void)
{
	return caches;
}

/*
 * Returns the cache with the given name. If no cache is found, `NULL` is
 * returned.
 */
ATTR_HOT
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

/*
 * Creates a cache named `name`, with objects of size `objsize`.
 * `objcount` is the number of objects per slab.
 * `ctor` is a function called at the construction of a new object.
 * `dtor` is a function called at the destruction of an object.
 */
ATTR_COLD
cache_t *cache_create(const char *name, size_t objsize, size_t objcount,
	void (*ctor)(void *, size_t), void (*dtor)(void *, size_t))
{
	cache_t *cache;

	if(!name || objsize == 0 || objcount == 0)
		return NULL;
	if(!(cache = cache_alloc(&caches_cache)))
		return NULL;
	cache->name = name;
	cache->objsize = objsize;
	cache->objcount = objcount;
	calc_slab_order(cache);
	cache->ctor = ctor;
	cache->dtor = dtor;
	cache->next = caches;
	caches = cache;
	return cache;
}

/*
 * Frees all slabs in the cache.
 * The cache structure might contain invalid references after calling this
 * function. It is meant to be used only before freeing the cache.
 */
ATTR_COLD
static void free_all_slabs(cache_t *cache, list_head_t *l)
{
	list_head_t *next;
	slab_t *s;

	while(l)
	{
		next = l->next;
		s = CONTAINER_OF(l, slab_t, list_node);
		buddy_free(s, cache->slab_order);
		l = next;
	}
}

/*
 * Destroyes the given cache.
 */
ATTR_COLD
void cache_destroy(cache_t *cache)
{
	cache_t *c;

	if(!cache)
		return;
	// TODO Use double linked list?
	c = caches;
	while(c)
	{
		if(c->next == cache)
		{
			c->next = c->next->next;
			break;
		}
		c = c->next;
	}
	free_all_slabs(cache, cache->slabs_full);
	free_all_slabs(cache, cache->slabs_partial);
	cache_free(&caches_cache, cache);
}

/*
 * Allocates a new slab for the given cache.
 */
ATTR_HOT
static list_head_t *alloc_slab(cache_t *cache)
{
	slab_t *slab;

	if(!(slab = buddy_alloc_zero(cache->slab_order)))
		return NULL;
	slab->available = cache->objcount;
	slab->node.value = (avl_value_t) slab;
	avl_tree_insert(&cache->tree, &slab->node, ptr_cmp);
	return &slab->list_node;
}

/*
 * Allocates an object on the given cache.
 */
ATTR_HOT
ATTR_MALLOC
void *cache_alloc(cache_t *cache)
{
	list_head_t *slab;
	slab_t *s;
	size_t i;
	void *ptr;

	if(!cache)
		return NULL;
	if(!(slab = cache->slabs_partial) && !(slab = alloc_slab(cache)))
		return NULL;
	s = CONTAINER_OF(slab, slab_t, list_node);
	i = bitfield_first_clear(s->use_bitfield, cache->objcount);
	bitfield_set(s->use_bitfield, i);
	--s->available;
	if(s->available == 0)
	{
		list_remove(&cache->slabs_partial, slab);
		list_remove(&cache->slabs_full, slab);
		list_insert_front(&cache->slabs_full, slab);
	}
	else if(s->available < cache->objcount)
	{
		list_remove(&cache->slabs_partial, slab);
		list_remove(&cache->slabs_full, slab);
		list_insert_front(&cache->slabs_partial, slab);
	}
	ptr = SLAB_OBJ(cache, s, i);
	if(cache->ctor)
		cache->ctor(ptr, cache->objsize);
	return ptr;
}

/*
 * Returns the slab for the given object.
 */
static slab_t *get_slab(cache_t *cache, void *obj)
{
	avl_tree_t *n;
	slab_t *s = NULL;

	n = cache->tree;
	while(n)
	{
		s = CONTAINER_OF(n, slab_t, node);
		if((void *) SLAB_OBJ(cache, s, cache->objcount) >= obj)
			n = n->left;
		else if((void *) SLAB_OBJ(cache, s, 0) < obj)
			n = n->right;
		else
			break;
	}
	if(!s)
		return NULL;
	if(obj < (void *) SLAB_OBJ(cache, s, 0))
		return NULL;
	if(obj >= (void *) SLAB_OBJ(cache, s, cache->objcount))
		return NULL;
	return s;
}

/*
 * Frees an object from the given cache.
 */
ATTR_HOT
void cache_free(cache_t *cache, void *obj)
{
	slab_t *s;
	size_t i;

	if(!cache || !obj)
		return;
	spin_lock(&cache->spinlock);
	if(!(s = get_slab(cache, obj)))
		goto end;
	i = (obj - (void *) SLAB_OBJ(cache, s, 0)) / cache->objsize;
	bitfield_clear(s->use_bitfield, i);
	if(s->available++ == 0)
	{
		list_remove(&cache->slabs_partial, &s->list_node);
		list_remove(&cache->slabs_full, &s->list_node);
		list_insert_front(&cache->slabs_partial, &s->list_node);
	}
	else if(s->available >= cache->objcount)
	{
		list_remove(&cache->slabs_partial, &s->list_node);
		list_remove(&cache->slabs_full, &s->list_node);
		buddy_free(s, cache->slab_order);
	}

end:
	spin_unlock(&cache->spinlock);
}
