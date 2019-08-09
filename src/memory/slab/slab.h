#ifndef SLAB_H
# define SLAB_H

# include <memory/memory.h>
# include <util/util.h>
# include <libc/string.h>

# define OBJ_USED	0b1

# define CACHES_CACHE_NAME	"slab_caches"

# define OBJ_TOTAL_SIZE(objsize)	(sizeof(object_t) + (objsize))
# define OBJ_FIRST(slab)			((object_t *) (slab) + sizeof(slab_t))
# define OBJ_CONTENT(ptr)			((void *) (ptr) + sizeof(object_t))

typedef uint8_t object_state;

typedef struct object
{
	struct object *next;
	object_state state;
} object_t;

typedef struct slab
{
	struct slab *next;
	object_t *first_object;
	object_t *last_object;
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

void slab_init(void);

cache_t *cache_getall(void);
cache_t *cache_get(const char *name);
cache_t *cache_create(const char *name, size_t objsize, size_t objects_count,
	void (*ctor)(void *, size_t), void (*dtor)(void *, size_t));
void cache_shrink(cache_t *cache);
void *cache_alloc(cache_t *cache);
void cache_free(cache_t *cache, void *obj);
void cache_destroy(cache_t *cache);

#endif
