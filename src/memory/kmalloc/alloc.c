#include <memory/kmalloc/kmalloc.h>
#include <memory/kmalloc/kmalloc_internal.h>
#include <util/util.h>

#include <libc/stdio.h> // TODO rm

/*
 * The kmalloc allocator works like the `malloc` function of the standard
 * library, allowing to allocate blocks of any size using functions `kmalloc`,
 * `kmalloc_zero`, `krealloc` and `kfree`.
 */

/*
 * Bins containing lists of free pages. The minimum size of chunks contained
 * into the bins is `KMALLOC_FREE_BIN_MIN << index`, where `index` is the index
 * of the bin.
 */
static list_head_t *free_bins[KMALLOC_FREE_BIN_COUNT];

/*
 * The bin for large free chunks.
 */
static list_head_t *large_bin = NULL;

/*
 * The spinlock for kmalloc.
 */
spinlock_t kmalloc_spinlock = 0;

#ifdef KMALLOC_MAGIC
/*
 * Asserts that the magic number for the given chunk is correct.
 */
static void check_magic(kmalloc_chunk_hdr_t *chunk)
{
	debug_assert(chunk, "kmalloc: invalid argument");
	debug_assert(chunk->magic == KMALLOC_MAGIC, "kmalloc: invalid argument");
}
#endif

/*
 * Returns a pointer to a free block that meets the required size. Returns
 * `NULL` if no block is found.
 */
static kmalloc_free_chunk_t *pop_free_chunk(size_t size)
{
	size_t i = 0;
	kmalloc_free_chunk_t *chunk;

	while(i < KMALLOC_FREE_BIN_COUNT)
	{
		if(size <= KMALLOC_BIN_SIZE(i) && free_bins[i])
			break;
		++i;
	}
	if(!free_bins[i])
		return NULL;
	chunk = CONTAINER_OF(free_bins[i], kmalloc_free_chunk_t, free_list);
	debug_assert(chunk->hdr.size >= size,
		"kmalloc: invalid chunk size for bin");
	list_remove(&free_bins[i], free_bins[i]);
	return chunk;
}

/*
 * Get free bin for the given `size`.
 */
static list_head_t **get_free_bin(size_t size)
{
	size_t i = 0;

	while(i < KMALLOC_FREE_BIN_COUNT && size > KMALLOC_BIN_SIZE(i + 1))
		++i;
	if(i >= KMALLOC_FREE_BIN_COUNT)
		return &large_bin;
	return &free_bins[i];
}

/*
 * Inserts the given chunk into its free bin.
 */
static void free_bin_insert(kmalloc_free_chunk_t *chunk)
{
	list_head_t **bin;

	debug_assert(sanity_check(chunk), "kmalloc: invalid argument");
	check_free_chunk(chunk);
	bin = get_free_bin(chunk->hdr.size);
	debug_assert(sanity_check(bin), "kmalloc: invalid bin");
	list_insert_front(bin, &chunk->free_list);
}

/*
 * Allocates a new block of memory suitable for the given size.
 */
static kmalloc_block_t *alloc_block(size_t size)
{
	size_t order;
	kmalloc_block_t *block;
	kmalloc_free_chunk_t *first_chunk;

	size = CEIL_DIVISION(sizeof(kmalloc_block_t) + size, PAGE_SIZE);
	order = buddy_get_order(size);
	size = FRAME_SIZE(order);
	if(!(block = buddy_alloc(order)))
		return NULL;
	block->buddy_order = order;
	first_chunk = (void *) &block->data;
	bzero(first_chunk, sizeof(kmalloc_free_chunk_t));
	first_chunk->hdr.magic = KMALLOC_MAGIC;
	first_chunk->hdr.block = block;
	first_chunk->hdr.size = size - sizeof(kmalloc_block_t);
	free_bin_insert(first_chunk);
	return block;
}

/*
 * Splits the given chunk according to the given `size`, creating a new free
 * chunk if large enough.
 */
static void consume_chunk(kmalloc_free_chunk_t *chunk, size_t size)
{
	kmalloc_free_chunk_t *new;

	debug_assert(chunk, "kmalloc: invalid argument");
#ifdef KERNEL_DEBUG
	check_free_chunk(chunk);
#endif
	debug_assert(chunk->hdr.size >= size,
		"kmalloc: block is too small for allocation");
	if(chunk->hdr.size >= size + sizeof(kmalloc_free_chunk_t) + KMALLOC_MIN)
	{
		new = (void *) (chunk + 1) + size;
#ifdef KMALLOC_MAGIC
		new->hdr.magic = KMALLOC_MAGIC;
#endif
		list_insert_after(NULL, &chunk->hdr.list, &new->hdr.list);
		new->hdr.block = chunk->hdr.block;
		new->hdr.size = chunk->hdr.size - size - sizeof(kmalloc_free_chunk_t);
		new->hdr.flags = 0;
		free_bin_insert(new);
		chunk->hdr.size = size;
	}
	chunk->hdr.flags |= KMALLOC_FLAG_USED;
}

/*
 * Allocates a block of the given size in bytes and retuns the pointer to the
 * beginning of it.
 */
void *alloc(size_t size)
{
	kmalloc_free_chunk_t *chunk;
	kmalloc_block_t *block;

	debug_assert(size > 0, "kmalloc: size == 0");
	if(!(chunk = pop_free_chunk(size)))
	{
		if(!(block = alloc_block(size)))
			return NULL;
		chunk = (void *) &block->data;
	}
	debug_assert(chunk->hdr.size >= size,
		"kmalloc: block is too small for allocation");
	consume_chunk(chunk, size);
	return &((kmalloc_used_chunk_t *) chunk)->data;
}

#ifdef KERNEL_DEBUG
/*
 * Asserts that the given chunk is valid. `bin` is the bin the chunk is stored
 * in.
 */
void check_free_chunk(kmalloc_free_chunk_t *chunk)
{
#ifdef KMALLOC_MAGIC
	check_magic(&chunk->hdr);
#endif
	debug_assert(!(chunk->hdr.flags & KMALLOC_FLAG_USED),
		"kmalloc: chunk should be free");
}

/*
 * Asserts that the given chunk is valid. `bin` is the bin the chunk is stored
 * in.
 */
void check_free_chunk_(kmalloc_free_chunk_t *chunk, size_t bin)
{
#ifdef KMALLOC_MAGIC
	check_magic(&chunk->hdr);
#endif
	debug_assert(!(chunk->hdr.flags & KMALLOC_FLAG_USED),
		"kmalloc: used chunk in free bin");
	debug_assert(chunk->hdr.size >= KMALLOC_BIN_SIZE(bin),
		"kmalloc: chunk is too small for its bin");
}

/*
 * Asserts that every chunks in the free bins are valid.
 */
void check_free_bins(void)
{
	size_t i;
	list_head_t *l;
	kmalloc_free_chunk_t *c;

	for(i = 0; i < KMALLOC_FREE_BIN_COUNT; ++i)
	{
		l = free_bins[i];
		while(l)
		{
			c = CONTAINER_OF(l, kmalloc_free_chunk_t, free_list);
			check_free_chunk_(c, i);
			l = l->next;
		}
	}
}
#endif
