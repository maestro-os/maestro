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
# define KMALLOC_MINIMUM	8

/*
 * Chunk flag telling that the chunk is allocated.
 */
# define KMALLOC_FLAG_USED	0b1

typedef struct
{
	/* Order of the block of memory */
	frame_order_t buddy_order;
	/* The data contained into the block */
	char data[0];
} kmalloc_block_t;

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
	/* The data into the chunk */
	char data[0];
} kmalloc_chunk_t;

extern spinlock_t kmalloc_spinlock;

void *alloc(size_t size);

#endif
