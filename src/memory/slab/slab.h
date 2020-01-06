#ifndef SLAB_H
# define SLAB_H

# include <memory/memory.h>
# include <util/util.h>

# define OBJ_USED	0b1

# define CACHES_CACHE_NAME	"caches"

# define SLAB_OBJ(cache, slab, i)	((slab)->use_bitfield\
	+ CEIL_DIVISION((cache)->objcount, 8) + (cache)->objsize * (i))

typedef struct slab
{
	struct slab *prev, *next;
	size_t available;

	uint8_t use_bitfield[0];
} slab_t;

typedef struct cache
{
	struct cache *next;

	const char *name;

	size_t slabs;
	size_t objsize;
	size_t objcount;

	size_t pages_per_slab;

	slab_t *slabs_full;
	slab_t *slabs_partial;
	avl_tree_t *tree;

	void (*ctor)(void *, size_t);
	void (*dtor)(void *, size_t);

	spinlock_t spinlock;
} cache_t;

void slab_init(void);

cache_t *cache_getall(void);
cache_t *cache_get(const char *name);
ATTR_MALLOC
cache_t *cache_create(const char *name, size_t objsize, size_t objcount,
	void (*ctor)(void *, size_t), void (*dtor)(void *, size_t));
void cache_destroy(cache_t *cache);

ATTR_MALLOC
void *cache_alloc(cache_t *cache);
void cache_free(cache_t *cache, void *obj);

#endif
