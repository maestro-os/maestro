#ifndef KMALLOC_INTERNAL_H
# define KMALLOC_INTERNAL_H

# include <memory/buddy/buddy.h>

# ifdef KERNEL_DEBUG
/*
 * The magic number to check integrity of memory chunks.
 */
#  define KMALLOC_MAGIC	0xdeadbeef
# endif

/*
 * Tells the minimum of available memory required to create a new chunk,
 * excluding the size of the header.
 */
# define KMALLOC_MIN\
	MAX(8, sizeof(kmalloc_free_chunk_t) - sizeof(kmalloc_chunk_hdr_t))

/*
 * The maximum size of an element in the first bin in bytes.
 */
# define KMALLOC_FREE_BIN_MIN		KMALLOC_MIN

/*
 * The number of free chunks bin.
 */
# define KMALLOC_FREE_BIN_COUNT		10

/*
 * Returns the minimum size in bytes for the given bin index.
 */
# define KMALLOC_BIN_SIZE(index)	(KMALLOC_FREE_BIN_MIN << (index))

/*
 * Chunk flag telling that the chunk is allocated.
 */
# define KMALLOC_FLAG_USED	0b1

/*
 * A block of memory to be divided into chunks
 */
typedef struct
{
	/* Order of the block of memory */
	frame_order_t buddy_order;
	/* The data contained into the block */
	char data[0];
} kmalloc_block_t;

/*
 * The header for chunks of memory
 */
typedef struct
{
# ifdef KMALLOC_MAGIC
	/* Magic number on every chunks, used to check integrity */
	uint32_t magic;
# endif

	/* The linked list for chunks of the same block */
	list_head_t list;
	/* Pointer to the block of memory the chunk is stored on */
	kmalloc_block_t *block;

	/* The size of the chunk, excluding the header */
	size_t size;
	/* Flags for the given chunk */
	char flags;
} kmalloc_chunk_hdr_t;

/*
 * A free chunk
 */
typedef struct
{
	/* The header for the chunk */
	kmalloc_chunk_hdr_t hdr;
	/* The free list in which the block is inserted */
	list_head_t free_list;
} kmalloc_free_chunk_t;

/*
 * A used chunk
 */
typedef struct
{
	/* The header for the chunk */
	kmalloc_chunk_hdr_t hdr;
	/* The data into the chunk */
	char data[0];
} kmalloc_used_chunk_t;

extern spinlock_t kmalloc_spinlock;

#ifdef KMALLOC_MAGIC
void check_magic(kmalloc_chunk_hdr_t *chunk);
#endif

list_head_t **get_free_bin(size_t size);
void free_bin_insert(kmalloc_free_chunk_t *chunk);
void free_bin_remove(kmalloc_free_chunk_t *chunk);

void *alloc(size_t size);

# ifdef KERNEL_DEBUG
int free_bin_has(kmalloc_free_chunk_t *chunk, list_head_t *bin);
int free_bins_has(kmalloc_free_chunk_t *chunk);
void check_free_chunk(kmalloc_free_chunk_t *chunk);
void check_free_chunk_(kmalloc_free_chunk_t *chunk, size_t bin);
void check_free_bins(void);
# endif

#endif
