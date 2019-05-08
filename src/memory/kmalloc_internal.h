#ifndef KMALLOC_INTERNAL
# define KMALLOC_INTERNAL

# include "memory.h"
# include "slab/slab.h"

cache_t *kmalloc_caches[7];

void kmalloc_init();

#endif
