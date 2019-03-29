#ifndef KALLOC_INTERNAL
# define KALLOC_INTERNAL

typedef struct mem_chunk
{
	void *ptr;
	size_t size;

	struct mem_chunk *next;
	struct mem_chunk *prev;
} mem_chunk_t;

// TODO

#endif
