#ifndef _KMALLOC_INTERNAL_H
# define _KMALLOC_INTERNAL_H

# include <util/util.h>

# define _FIRST_SMALL_BUCKET_SIZE	((size_t) 8)
# define _SMALL_BUCKETS_COUNT		((size_t) 6)

# define _SMALL_BIN_MAX\
	(_FIRST_SMALL_BUCKET_SIZE << (_SMALL_BUCKETS_COUNT - 1))

# define _FIRST_MEDIUM_BUCKET_SIZE	_SMALL_BIN_MAX
# define _MEDIUM_BUCKETS_COUNT		((size_t) 11)

# define _MEDIUM_BIN_MAX			((size_t) 262144)

# define _SMALL_BLOCK_PAGES		((size_t) 8)
# define _MEDIUM_BLOCK_PAGES	((size_t) 128)

/*
 * Chunks alignment boundary.
 */
# define ALIGNMENT				16
/*
 * Magic number used to check integrity of memory chunks.
 */
# define _MALLOC_CHUNK_MAGIC	(0x5ea310c36f405b33 & (sizeof(long) == 8\
	? ~((unsigned long) 0) : 0xffffffff))

# define BLOCK_DATA(b)		ALIGN(((_block_t *) (b))->data, ALIGNMENT)
# define CHUNK_DATA(c)		ALIGN(((_used_chunk_t *) (c))->data, ALIGNMENT)

# define BLOCK_HDR_SIZE		((size_t) BLOCK_DATA(NULL))
# define CHUNK_HDR_SIZE		((size_t) CHUNK_DATA(NULL))

# define GET_CHUNK(ptr)		((void *) ((void *) (ptr) - CHUNK_DATA(NULL)))

typedef struct _block _block_t;

/*
 * Memory chunk header
 */
typedef struct _chunk_hdr
{
	struct _chunk_hdr *prev, *next;

	_block_t *block;
	size_t size;
	char used;
# ifdef _MALLOC_CHUNK_MAGIC
	long magic;
# endif
} _chunk_hdr_t;

/*
 * Used chunk structure
 */
typedef struct _used_chunk
{
	_chunk_hdr_t hdr;
	char data[0];
} _used_chunk_t;

/*
 * Free chunk structure
 */
typedef struct _free_chunk
{
	_chunk_hdr_t hdr;
	struct _free_chunk *prev_free, *next_free;
} _free_chunk_t;

/*
 * Memory block structure
 */
typedef struct _block
{
	struct _block *prev, *next;

	size_t pages;
	char data[0];
} _block_t;

_block_t *_alloc_block(const size_t pages);
_block_t **_block_get_bin(_block_t *b);
void _free_block(_block_t *b);

void _bucket_link(_free_chunk_t *chunk);
void _bucket_unlink(_free_chunk_t *chunk);
_free_chunk_t **_get_bucket(size_t size, int insert, int medium);

void _split_chunk(_chunk_hdr_t *chunk, size_t size);
void _merge_chunks(_chunk_hdr_t *c);
void _alloc_chunk(_free_chunk_t *chunk, size_t size);

void *_small_alloc(size_t size);
void *_medium_alloc(size_t size);
void *_large_alloc(size_t size);

void _chunk_assert(_chunk_hdr_t *c);

extern spinlock_t kmalloc_spinlock;

#endif
