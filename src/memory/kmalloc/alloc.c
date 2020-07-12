#include <memory/kmalloc/kmalloc.h>
#include <memory/kmalloc/kmalloc_internal.h>
#include <util/util.h>

/*
 * The kmalloc allocator works like the `malloc` function of the standard
 * library, allowing to allocate blocks of any size using functions `kmalloc`,
 * `kmalloc_zero`, `krealloc` and `kfree`.
 */

/*
 * The spinlock for kmalloc.
 */
spinlock_t kmalloc_spinlock = 0;

#ifdef KMALLOC_MAGIC
/*
 * Asserts that the magic number for the given chunk is correct.
 */
static void check_magic(kmalloc_chunk_t *chunk)
{
	debug_assert(chunk, "kmalloc: invalid argument");
	debug_assert(chunk->magic == KMALLOC_MAGIC, "kmalloc: invalid argument");
}
#endif

/*
 * Returns a pointer to a free block that meets the required size. Returns
 * `NULL` if no block is found.
 */
static kmalloc_chunk_t *get_free_chunk(size_t size)
{
	// TODO
	(void) size;
	return NULL;
}

/*
 * Allocates a new block of memory suitable for the given size.
 */
static kmalloc_block_t *alloc_block(size_t size)
{
	size_t order;
	kmalloc_block_t *block;
	kmalloc_chunk_t *first_chunk;

	size = CEIL_DIVISION(sizeof(kmalloc_block_t) + size, PAGE_SIZE);
	order = buddy_get_order(size);
	size = FRAME_SIZE(order);
	if(!(block = buddy_alloc(order)))
		return NULL;
	block->buddy_order = order;
	first_chunk = (void *) &block->data;
	bzero(first_chunk, sizeof(kmalloc_chunk_t));
	first_chunk->magic = KMALLOC_MAGIC;
	first_chunk->block = block;
	first_chunk->size = size - sizeof(kmalloc_block_t);
	return block;
}

/*
 * Splits the given chunk according to the given `size`, creating a new free
 * chunk if large enough.
 */
static void consume_chunk(kmalloc_chunk_t *chunk, size_t size)
{
	kmalloc_chunk_t *new;

	debug_assert(chunk, "kmalloc: invalid argument");
#ifdef KMALLOC_MAGIC
	check_magic(chunk);
#endif
	debug_assert(!(chunk->flags & KMALLOC_FLAG_USED),
		"kmalloc: trying to consume used chunk");
	debug_assert(chunk->size < size,
		"kmalloc: block is too small for allocation");
	if(chunk->size >= size + sizeof(kmalloc_chunk_t) + KMALLOC_MINIMUM)
	{
		new = (void *) (chunk + 1) + size;
#ifdef KMALLOC_MAGIC
		new->magic = KMALLOC_MAGIC;
#endif
		list_insert_after(NULL, &chunk->list, &new->list);
		new->block = chunk->block;
		new->size = chunk->size - size - sizeof(kmalloc_chunk_t);
		new->flags = 0;
		chunk->size = size;
	}
	chunk->flags |= KMALLOC_FLAG_USED;
}

/*
 * Allocates a block of the given size in bytes and retuns the pointer to the
 * beginning of it.
 */
void *alloc(size_t size)
{
	kmalloc_chunk_t *chunk;
	kmalloc_block_t *block;

	debug_assert(size > 0, "kmalloc: size == 0");
	if(!(chunk = get_free_chunk(size)))
	{
		if(!(block = alloc_block(size)))
			return NULL;
		chunk = (void *) &block->data;
	}
	consume_chunk(chunk, size);
	return &chunk->data;
}
