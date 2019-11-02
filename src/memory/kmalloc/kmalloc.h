#ifndef KMALLOC_H
# define KMALLOC_H

# include <memory/memory.h>
# include <util/util.h>

# define CHUNK_HEAD(ptr)	(CONTAINER_OF(ptr, chunk_t, content))

# define BUCKETS_COUNT	6
# define SMALLER_BUCKET	8

# define CHUNK_FLAG_USED	0b01
# define CHUNK_FLAG_BUDDY	0b10

# define CHUNK_IS_USED(chunk)	((chunk)->flags & CHUNK_FLAG_USED)

# define KMALLOC_BUDDY	0b1

typedef struct chunk
{
	struct chunk *prev;
	struct chunk *next;

	size_t size;
	int8_t flags;

	char content[0];
} chunk_t;

extern spinlock_t kmalloc_spinlock;

chunk_t *get_chunk(void *ptr);
chunk_t *get_free_chunk(size_t size, int flags);
void alloc_chunk(chunk_t *chunk, size_t size);
void free_chunk(chunk_t *chunk, int flags);

void *kmalloc(size_t size, int flags);
void *kmalloc_zero(size_t size, int flags);
void *krealloc(void *ptr, size_t size, int flags);
void kfree(void *ptr, int flags);

#endif
