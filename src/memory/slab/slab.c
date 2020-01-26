#include <kernel.h>
#include <memory/slab/slab.h>
#include <libc/errno.h>

static cache_t *caches;
static cache_t *caches_cache;

ATTR_HOT
static void calc_pages_per_slab(cache_t *cache)
{
	cache->pages_per_slab = CEIL_DIVISION(sizeof(slab_t)
		+ CEIL_DIVISION(cache->objcount, 8)
			+ (cache->objsize * cache->objcount), PAGE_SIZE);
}

ATTR_COLD
void slab_init(void)
{
	if(!(caches_cache = kmalloc_zero(sizeof(cache_t))))
		PANIC("Failed to initialize slab allocator!", 0);
	caches_cache->name = CACHES_CACHE_NAME;
	caches_cache->objsize = sizeof(cache_t);
	caches_cache->objcount = 32;
	calc_pages_per_slab(caches_cache);
	caches_cache->ctor = bzero;
	caches = caches_cache;
}

ATTR_HOT
cache_t *cache_getall(void)
{
	return caches;
}

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

ATTR_COLD
cache_t *cache_create(const char *name, size_t objsize, size_t objcount,
	void (*ctor)(void *, size_t), void (*dtor)(void *, size_t))
{
	cache_t *cache;

	if(!name || objsize == 0 || objcount == 0)
		return NULL;
	if(!(cache = cache_alloc(caches_cache)))
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

ATTR_COLD
static void free_all_slabs(slab_t *s)
{
	slab_t *next;

	while(s)
	{
		next = s->next;
		pages_free(s);
		s = next;
	}
}

ATTR_COLD
void cache_destroy(cache_t *cache)
{
	cache_t *c;

	if(!cache)
		return;
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
	free_all_slabs(cache->slabs_full);
	free_all_slabs(cache->slabs_partial);
	avl_tree_freeall(&cache->tree, NULL);
	cache_free(caches_cache, cache);
}

ATTR_HOT
static void link_slab(slab_t **list, slab_t *slab)
{
	if(!slab)
		return;
	if((slab->next = *list))
		slab->next->prev = slab;
	slab->prev = NULL;
	*list = slab;
}

ATTR_HOT
static void unlink_slab(cache_t *cache, slab_t *slab)
{
	if(!slab)
		return;
	if(slab == cache->slabs_full)
		cache->slabs_full = slab->next;
	if(slab == cache->slabs_partial)
		cache->slabs_partial = slab->next;
	if(slab->next)
		slab->next->prev = slab->prev;
	if(slab->prev)
		slab->prev->next = slab->next;
}

ATTR_HOT
static slab_t *alloc_slab(cache_t *cache)
{
	slab_t *slab;

	if(!(slab = pages_alloc_zero(cache->pages_per_slab)))
		return NULL;
	slab->available = cache->objcount;
	avl_tree_insert(&cache->tree, slab, ptr_cmp);
	return slab;
}

ATTR_HOT
ATTR_MALLOC
void *cache_alloc(cache_t *cache)
{
	slab_t *slab;
	size_t i;
	void *ptr;

	if(!cache)
		return NULL;
	if(!(slab = cache->slabs_partial) && !(slab = alloc_slab(cache)))
		return NULL;
	i = bitfield_first_clear(slab->use_bitfield, cache->objcount);
	bitfield_set(slab->use_bitfield, i);
	--slab->available;
	if(slab->available == 0)
	{
		unlink_slab(cache, slab);
		link_slab(&cache->slabs_full, slab);
	}
	else if(slab->available < cache->objcount)
	{
		unlink_slab(cache, slab);
		link_slab(&cache->slabs_partial, slab);
	}
	ptr = SLAB_OBJ(cache, slab, i);
	if(cache->ctor)
		cache->ctor(ptr, cache->objsize);
	return ptr;
}

static avl_tree_t *get_slab(cache_t *cache, void *obj)
{
	slab_t *s = NULL;
	avl_tree_t *n;

	n = cache->tree;
	while(n)
	{
		s = n->value;
		if((void *) SLAB_OBJ(cache, s, cache->objcount) >= obj)
			n = n->left;
		else if((void *) SLAB_OBJ(cache, s, 0) < obj)
			n = n->right;
		else
			break;
	}
	if(!n)
		return NULL;
	if(obj < (void *) SLAB_OBJ(cache, s, 0))
		return NULL;
	if(obj >= (void *) SLAB_OBJ(cache, s, cache->objcount))
		return NULL;
	return n;
}

ATTR_HOT
void cache_free(cache_t *cache, void *obj)
{
	avl_tree_t *n;
	slab_t *s;
	size_t i;

	if(!cache || !obj)
		return;
	spin_lock(&cache->spinlock);
	if(!(n = get_slab(cache, obj)))
		goto end;
	s = n->value;
	i = (obj - (void *) SLAB_OBJ(cache, s, 0)) / cache->objsize;
	bitfield_clear(s->use_bitfield, i);
	if(s->available++ == 0)
	{
		unlink_slab(cache, s);
		link_slab(&cache->slabs_partial, s);
	}
	else if(s->available >= cache->objcount)
	{
		unlink_slab(cache, s);
		avl_tree_delete(&cache->tree, n);
		pages_free(s);
	}

end:
	spin_unlock(&cache->spinlock);
}
