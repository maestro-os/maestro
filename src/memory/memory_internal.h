#ifndef MEMORY_INTERNAL_H
# define MEMORY_INTERNAL_H

# define CHK_ISFREE(chunk)	(chunk->flags == 0)

typedef uint8_t chunk_flags_t;

typedef struct mem_chunk_tag
{
	size_t size;
	chunk_flags_t flags;

	struct mem_chunk *next;
	struct mem_chunk *prev;
} __attribute__((packed)) mem_chunk_tag_t;

#endif
