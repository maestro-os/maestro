#ifndef ALLOC_INTERNAL_H
# define ALLOC_INTERNAL_H

# define LARGE_THRESHOLD	// TODO

typedef struct malloc_chunk
{
	size_t size;

	struct malloc_chunk *next;
	struct malloc_chunk *prev;
} malloc_chunk_t;

malloc_chunk_t *new_chunks();
// TODO

#endif
