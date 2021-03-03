#ifndef SLAB_H
# define SLAB_H

# include <memory/memory.h>
# include <util/util.h>

/*
 * Flags telling that the object is used.
 */
# define OBJ_USED	0b1

/*
 * Name of the cache that contains all caches.
 */
# define CACHES_CACHE_NAME	"caches"

/*
 * Gives the pointer to the object in the specified cache, slab and index.
 */
# define SLAB_OBJ(cache, slab, i)	((slab)->use_bitfield\
	+ CEIL_DIVISION((cache)->objcount, 8) + (cache)->objsize * (i))

/*
 * Structure representing a slab.
 */
typedef struct slab
{
	/* Double-linked list of slabs. */
	list_head_t list_node;
	/* The node of the tree the slab is located in. */
	avl_tree_t node;

	/* The amount of available objects in the slab. */
	size_t available;
	/* Bitfield telling which object is used in the slab. */
	uint8_t use_bitfield[0];
} slab_t;

typedef struct cache
{
	/* Linked list of caches. */
	struct cache *next;

	/* The name of the cache. */
	const char *name;

	/* The size of an object. */
	size_t objsize;
	/* The number of objects contained in one slab. */
	size_t objcount;

	/* The order of a memory block for a slab */
	size_t slab_order;

	/* The list of full slabs */
	list_head_t *slabs_full;
	/* The list of partial slabs (some objects are still available) */
	list_head_t *slabs_partial;
	/* The tree containing all the slabs for fast retrieval */
	avl_tree_t *tree;

	/* The constructor function for the objects of the cache. */
	void (*ctor)(void *, size_t);
	/* The destructor function for the objects of the cache. */
	void (*dtor)(void *, size_t);

	/* The spinlock of the cache. */
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
