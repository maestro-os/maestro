#ifndef KMALLOC_INTERNAL
# define KMALLOC_INTERNAL

# include "memory.h"
# include "slab/slab.h"

# define KMALLOC_CACHES_COUNT	7

cache_t *kmalloc_caches[KMALLOC_CACHES_COUNT];

void kmalloc_init();

#endif
