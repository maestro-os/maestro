#ifndef SLAB_H
# define SLAB_H

# include "../memory.h"
# include "../../util/util.h"
# include "../../libc/string.h"

# define OBJ_FREE	0b0
# define OBJ_USED	0b1

# define SLAB_CACHE_NAME	"slab_caches"

typedef uint8_t object_state;

typedef struct object
{
	object_state state;
} object_t;

typedef struct free_object
{
	object_state state;
	object_t *next;
} free_object_t;

// TODO Use a linked list of slabs in order to be able to extend caches?
typedef struct slabs
{
	object_t *objs;
	size_t used;
} slabs_t;

typedef struct cache
{
	const char *name;

	size_t objsize;
	size_t objects_count;
	size_t slabs;

	slabs_t slabs_full;
	slabs_t slabs_partial;
	slabs_t slabs_free;

	void (*ctor)(void *, const size_t);
	void (*dtor)(void *, const size_t);

	spinlock_t spinlock;
	struct cache *next;
} cache_t;

void slab_init();

cache_t *cache_getall();
cache_t *cache_get(const char *name);
cache_t *cache_create(const char *name, const size_t objsize,
	const size_t objects_count, void (*ctor)(void *, size_t),
		void (*dtor)(void *, size_t));
void cache_shrink(cache_t *cache);
void *cache_alloc(cache_t *cache);
void cache_free(cache_t *cache, void *obj);
void cache_destroy(cache_t *cache);

#endif
