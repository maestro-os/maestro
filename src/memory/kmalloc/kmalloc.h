#ifndef KMALLOC_H
# define KMALLOC_H

# include <memory/memory.h>
# include <util/util.h>

# define CHUNK_CONTENT(chunk)	((void *) (chunk) + sizeof(chunk_t))

# define BUCKETS_COUNT	6
# define SMALLER_BUCKET	8

typedef struct chunk
{
	struct chunk *prev;
	struct chunk *next;
	size_t size;
	uint8_t used;
} chunk_t;

extern spinlock_t kmalloc_spinlock;
// TODO Use flags on function calls instead?
extern int kmalloc_buddy;

chunk_t *get_chunk(void *ptr);
chunk_t *get_free_chunk(size_t size);
void alloc_chunk(chunk_t *chunk, size_t size);
void free_chunk(chunk_t *chunk);

void *kmalloc(size_t size);
void *kmalloc_zero(size_t size);
void *krealloc(void *ptr, size_t size);
void kfree(void *ptr);

#endif
