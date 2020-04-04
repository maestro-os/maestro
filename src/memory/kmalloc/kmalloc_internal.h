#ifndef KMALLOC_INTERNAL_H
# define KMALLOC_INTERNAL_H

# include <util/util.h>

# define FIRST_SMALL_BUCKET_SIZE	((size_t) 8)
# define SMALL_BUCKETS_COUNT		((size_t) 6)

# define SMALL_BIN_MAX	(FIRST_SMALL_BUCKET_SIZE << (SMALL_BUCKETS_COUNT - 1))

# define FIRST_MEDIUM_BUCKET_SIZE	SMALL_BIN_MAX
# define MEDIUM_BUCKETS_COUNT		((size_t) 11)

# define MEDIUM_BIN_MAX				((size_t) 262144)

# define SMALL_BLOCK_PAGES			((size_t) 8)
# define MEDIUM_BLOCK_PAGES			((size_t) 128)

/*
 * Chunks alignment boundary.
 */
# define ALIGNMENT	16
/*
 * Magic number used to check integrity of memory chunks.
 */
# define _MALLOC_CHUNK_MAGIC	(0x5ea310c36f405b33 & (sizeof(long) == 8\
	? ~((unsigned long) 0) : 0xffffffff))

# define BLOCK_DATA(b)		ALIGN(((block_t *) (b))->data, ALIGNMENT)
# define CHUNK_DATA(c)		ALIGN(((used_chunk_t *) (c))->data, ALIGNMENT)

# define BLOCK_HDR_SIZE		((size_t) BLOCK_DATA(NULL))
# define CHUNK_HDR_SIZE		((size_t) CHUNK_DATA(NULL))

# define GET_CHUNK(ptr)		((void *) ((void *) (ptr) - CHUNK_DATA(NULL)))

typedef struct block block_t;

/*
 * Memory chunk header
 */
typedef struct chunk_hdr
{
	struct chunk_hdr *prev, *next;

	block_t *block;
	size_t size;
	char used;
# ifdef MALLOC_CHUNK_MAGIC
	long magic;
# endif
} chunk_hdr_t;

/*
 * Used chunk structure
 */
typedef struct used_chunk
{
	chunk_hdr_t hdr;
	char data[0];
} used_chunk_t;

/*
 * Free chunk structure
 */
typedef struct free_chunk
{
	chunk_hdr_t hdr;
	struct free_chunk *prev_free, *next_free;
} free_chunk_t;

/*
 * Memory block structure
 */
typedef struct block
{
	struct block *prev, *next;

	size_t pages;
	char data[0];
} block_t;

block_t *kmalloc_alloc_block(const size_t pages);
block_t **block_get_bin(block_t *b);
void kmalloc_free_block(block_t *b);

void bucket_link(free_chunk_t *chunk);
void bucket_unlink(free_chunk_t *chunk);
free_chunk_t **get_bucket(size_t size, int insert, int medium);

void split_chunk(chunk_hdr_t *chunk, size_t size);
void merge_chunks(chunk_hdr_t *c);
void alloc_chunk(free_chunk_t *chunk, size_t size);

void *small_alloc(size_t size);
void *medium_alloc(size_t size);
void *large_alloc(size_t size);

void chunk_assert(chunk_hdr_t *c);

extern spinlock_t kmalloc_spinlock;

#endif
