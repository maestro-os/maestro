#ifndef SLAB_H
# define SLAB_H

# include "../memory.h"
# include "../../util/util.h"
# include "../../libc/string.h"

# define OBJ_USED	0b1

# define CACHES_CACHE_NAME	"slab_caches"
# define CACHES_CACHE_ORDER	0

# define OBJ_TOTAL_SIZE(objsize)	(sizeof(object_t) + (objsize))
# define OBJ_CONTENT(ptr)			((ptr) + sizeof(object_t))
# define OBJ_NEXT(curr, size)		((curr) + OBJ_TOTAL_SIZE(size))

typedef uint8_t object_state;

typedef struct object
{
	object_state state;
	// TODO
} object_t;

typedef struct slab
{
	object_t *free_objs;
	size_t used;

	struct slab *next;
} slab_t;

typedef struct cache
{
	const char *name;
	spinlock_t spinlock;

	size_t slabs;
	size_t objsize;
	size_t objects_count;

	slab_t *slabs_full;
	slab_t *slabs_partial;
	slab_t *slabs_free;

	void (*ctor)(void *, size_t);
	void (*dtor)(void *, size_t);

	struct cache *next;
} cache_t;

void slab_init();

cache_t *cache_getall();
cache_t *cache_get(const char *name);
cache_t *cache_create(const char *name, size_t objsize, size_t objects_count,
	void (*ctor)(void *, size_t), void (*dtor)(void *, size_t));
void cache_shrink(cache_t *cache);
void *cache_alloc(cache_t *cache);
void cache_free(cache_t *cache, void *obj);
void cache_destroy(cache_t *cache);

#endif
