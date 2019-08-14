#ifndef SLAB_H
# define SLAB_H

# include <memory/memory.h>
# include <util/util.h>

# define OBJ_USED	0b1

# define CACHES_CACHE_NAME	"slab_caches"

# define SLAB_BITMAP(slab)			((void *) (slab) + sizeof(slab_t))
# define SLAB_OBJ(cache, slab, i)	(SLAB_BITMAP(slab)\
	+ UPPER_DIVISION((cache)->objcount, 8) + (cache)->objsize * (i))

typedef struct slab
{
	struct slab *prev, *next;
	size_t available;
} slab_t;

typedef struct cache
{
	const char *name;
	spinlock_t spinlock;

	size_t slabs;
	size_t objsize;
	size_t objcount;

	size_t pages_per_slab;

	slab_t *slabs_full;
	slab_t *slabs_partial;
	slab_t *slabs_free;

	void (*ctor)(void *, size_t);
	void (*dtor)(void *, size_t);

	struct cache *next;
} cache_t;

void slab_init(void);

cache_t *cache_getall(void);
cache_t *cache_get(const char *name);
cache_t *cache_create(const char *name, size_t objsize, size_t objcount,
	void (*ctor)(void *, size_t), void (*dtor)(void *, size_t));
void *cache_alloc(cache_t *cache);
void cache_shrink(cache_t *cache);
void cache_free(cache_t *cache, void *obj);
void cache_destroy(cache_t *cache);

#endif
