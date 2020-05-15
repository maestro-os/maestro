#ifndef KMALLOC_INTERNAL_H
# define KMALLOC_INTERNAL_H

# include <util/util.h>

# define FIRST_BUCKET_SIZE	((size_t) 8)
# define BUCKETS_COUNT		((size_t) 6)

/*
 * Chunks alignment boundary.
 */
# define ALIGNMENT	16

# ifdef KERNEL_DEBUG
/*
 * Magic number used to check integrity of memory chunks.
 */
#  define MALLOC_CHUNK_MAGIC	(0x5ea310c36f405b33 & (sizeof(long) == 8\
	? ~((unsigned long) 0) : 0xffffffff))
# endif

# define BLOCK_DATA(b)		ALIGN(((block_t *) (b))->data, ALIGNMENT)
# define CHUNK_DATA(c)		ALIGN(((used_chunk_t *) (c))->data, ALIGNMENT)

# define BLOCK_HDR_SIZE		((size_t) BLOCK_DATA(NULL))
# define CHUNK_HDR_SIZE		((size_t) CHUNK_DATA(NULL))

# define GET_CHUNK(ptr)		((void *) ((void *) (ptr) - CHUNK_DATA(NULL)))

// TODO Use list_head_t
// TODO Heavy selftesting

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

	size_t buddy_order;
	char data[0];
} block_t;

block_t *kmalloc_alloc_block(const size_t pages);
void kmalloc_free_block(block_t *b);

void bucket_link(free_chunk_t *chunk);
void bucket_unlink(free_chunk_t *chunk);
free_chunk_t **get_bucket(size_t size, int insert);

void split_chunk(chunk_hdr_t *chunk, size_t size);
void merge_chunks(chunk_hdr_t *c);
void alloc_chunk(free_chunk_t *chunk, size_t size);

void *small_alloc(size_t size);
void *medium_alloc(size_t size);
void *large_alloc(size_t size);

void chunk_assert(chunk_hdr_t *c);

extern spinlock_t kmalloc_spinlock;

#endif
