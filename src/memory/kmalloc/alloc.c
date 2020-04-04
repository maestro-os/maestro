#include <kernel.h>
#include <memory/kmalloc/kmalloc_internal.h>
#include <memory/pages/pages.h>

// TODO Do not use printf/dprintf for errors printing

/*
 * This file handles internal operations for the allocator.
 *
 * Allocations are stored into linked lists of chunks. Every chunk is allocated
 * according to the value into `ALIGNMENT`.
 *
 * Chunks are contained into blocks which represent the memory regions given by
 * the kernel.
 * Blocks are sorted into bins according to the size of the allocations they can
 * handle.
 *
 * - small_bin: size < SMALL_BIN_MAX
 * - medium_bin: size < MEDIUM_BIN_MAX
 * - large_bin: size >= MEDIUM_BIN_MAX
 *
 * Blocks in `large_bin` contain only one allocation which can be several
 * pages large.
 *
 * MALLOC_CHUNK_MAGIC is a magic number used in chunk structures to ensure that
 * chunks aren't overwritten. If the value has been changed between two
 * operations of the allocator, the current process shall abort.
 */

/*
 * Bins containing the list of allocated blocks.
 */
block_t *small_bin = NULL;
block_t *medium_bin = NULL;
block_t *large_bin = NULL;

/*
 * Buckets containing lists of free chunks.
 * Lists are sorted according to the size of the empty chunk.
 *
 * A chunk must be at least `n` bytes large to fit in a bucket, where
 * n=_FIRST_SMALL_BUCKET_SIZE * 2^i . Here, `i` is the index in the array.
 */
ATTR_BSS
free_chunk_t *small_buckets[SMALL_BUCKETS_COUNT];
ATTR_BSS
free_chunk_t *medium_buckets[MEDIUM_BUCKETS_COUNT];

spinlock_t kmalloc_spinlock;

/*
 * Links the given block to the given bin.
 */
static inline void bin_link(block_t *block)
{
	block_t **bin;

	bin = block_get_bin(block);
	if((block->next = *bin))
		block->next->prev = block;
	*bin = block;
}

/*
 * Allocates a `pages` pages long block of memory and creates a chunk on it that
 * covers the whole block.
 */
ATTR_MALLOC
block_t *kmalloc_alloc_block(const size_t pages)
{
	block_t *b;
	chunk_hdr_t *first_chunk;

	if(pages == 0 || !(b = pages_alloc(pages)))
		return NULL;
	bzero(b, BLOCK_HDR_SIZE);
	b->pages = pages;
	first_chunk = BLOCK_DATA(b);
	first_chunk->block = b;
	first_chunk->size = pages * PAGE_SIZE - (BLOCK_HDR_SIZE + CHUNK_HDR_SIZE);
#ifdef MALLOC_CHUNK_MAGIC
	first_chunk->magic = MALLOC_CHUNK_MAGIC;
#endif
	bin_link(b);
	return b;
}

/*
 * Returns a pointer to the bin for the given block.
 */
block_t **block_get_bin(block_t *b)
{
	if(b->pages <= SMALL_BLOCK_PAGES)
		return &small_bin;
	else if(b->pages <= MEDIUM_BLOCK_PAGES)
		return &medium_bin;
	return &large_bin;
}

/*
 * Unlinks the given block `b` from its bin and frees it.
 */
void kmalloc_free_block(block_t *b)
{
	if(b->prev)
		b->prev->next = b->next;
	else if(b == small_bin)
		small_bin = b->next;
	else if(b == medium_bin)
		medium_bin = b->next;
	else if(b == large_bin)
		large_bin = b->next;
	if(b->next)
		b->next->prev = b->prev;
	pages_free(b, b->pages);
}

/*
 * Returns a small bucket containing chunks large enough to fit an allocation of
 * the given `size`. If `insert` is not zero, the function will return the first
 * bucket that fits even if empty to allow insertion of a new free chunk.
 */
free_chunk_t **get_bucket(const size_t size, const int insert, const int medium)
{
	free_chunk_t **buckets;
	size_t first, count;
	size_t i = 0;

	if(medium)
	{
		buckets = medium_buckets;
		first = FIRST_MEDIUM_BUCKET_SIZE;
		count = MEDIUM_BUCKETS_COUNT;
	}
	else
	{
		buckets = small_buckets;
		first = FIRST_SMALL_BUCKET_SIZE;
		count = SMALL_BUCKETS_COUNT;
	}
	if(size < first)
		return NULL;
	if(insert)
		while(size >= (first << (i + 1)) && i < count - 1)
			++i;
	else
		while((!buckets[i] || size > (first << i)) && i < count - 1)
			++i;
	return buckets + i;
}

/*
 * Links the given free chunk to the corresponding bucket.
 */
void bucket_link(free_chunk_t *chunk)
{
	free_chunk_t **bucket;

	if(!(bucket = get_bucket(chunk->hdr.size, 1,
		block_get_bin(chunk->hdr.block) == &medium_bin)))
		return;
	chunk->prev_free = NULL;
	if((chunk->next_free = *bucket))
		chunk->next_free->prev_free = chunk;
	*bucket = chunk;
}

/*
 * Unlinks the given free chunk from its bucket.
 */
void bucket_unlink(free_chunk_t *chunk)
{
	size_t i;

	// TODO Check block type instead of checking both small and medium?
	for(i = 0; i < SMALL_BUCKETS_COUNT; ++i)
		if(small_buckets[i] == chunk)
			small_buckets[i] = chunk->next_free;
	for(i = 0; i < MEDIUM_BUCKETS_COUNT; ++i)
		if(medium_buckets[i] == chunk)
			medium_buckets[i] = chunk->next_free;
	if(chunk->prev_free)
		chunk->prev_free->next_free = chunk->next_free;
	if(chunk->next_free)
		chunk->next_free->prev_free = chunk->prev_free;
	chunk->prev_free = NULL;
	chunk->next_free = NULL;
}

/*
 * Splits the given chunk into two chunks. The first chunk will take size `size`
 * and the second chunk will take the remaining space.
 * Note that the function might do nothing if the chunk isn't large enough to be
 * split.
 */
void split_chunk(chunk_hdr_t *chunk, size_t size)
{
	free_chunk_t *new_chunk;
	size_t l;

	size = MAX(ALIGNMENT, size);
	new_chunk = (free_chunk_t *) ALIGN(CHUNK_DATA(chunk) + size, ALIGNMENT);
	if(chunk->size <= size + CHUNK_HDR_SIZE + ALIGNMENT)
		return;
	l = chunk->size;
	chunk->size = size;
	if(!chunk->used)
	{
		bucket_unlink((free_chunk_t *) chunk);
		bucket_link((free_chunk_t *) chunk);
	}
	if((new_chunk->hdr.next = (chunk_hdr_t *) chunk->next))
		new_chunk->hdr.next->prev = (chunk_hdr_t *) new_chunk;
	if((new_chunk->hdr.prev = (chunk_hdr_t *) chunk))
		new_chunk->hdr.prev->next = (chunk_hdr_t *) new_chunk;
	new_chunk->hdr.block = chunk->block;
	new_chunk->hdr.size = l - (size + CHUNK_HDR_SIZE);
	new_chunk->hdr.used = 0;
#ifdef MALLOC_CHUNK_MAGIC
	new_chunk->hdr.magic = MALLOC_CHUNK_MAGIC;
#endif
	bucket_link(new_chunk);
}

/*
 * Merges the given chunk with the following chunk.
 */
void merge_chunks(chunk_hdr_t *c)
{
	if(!c->next->used)
		bucket_unlink((free_chunk_t *) c->next);
	c->size += CHUNK_HDR_SIZE + c->next->size;
	if((c->next = c->next->next))
		c->next->prev = c;
}

/*
 * Allocates the given chunk for size `size`. The given chunk must be large
 * enough to fit the allocation. Chunk might be split to another free chunk
 * if large enough. The new free chunk might be inserted in buckets for
 * further allocations.
 */
void alloc_chunk(free_chunk_t *chunk, const size_t size)
{
#ifdef MALLOC_CHUNK_MAGIC
	assert(chunk->hdr.magic == _MALLOC_CHUNK_MAGIC, "kmalloc: corrupted chunk");
#endif
	assert(!chunk->hdr.used && chunk->hdr.size >= size,
		"kmalloc: internal error");
	bucket_unlink(chunk);
	chunk->hdr.used = 1;
	split_chunk(&chunk->hdr, size);
}

/*
 * Checks that the given chunk is valid for reallocation/freeing.
 * If the chunk is invalid, prints an error message and aborts.
 */
void chunk_assert(chunk_hdr_t *c)
{
#ifdef MALLOC_CHUNK_MAGIC
	assert(c->magic == MALLOC_CHUNK_MAGIC, "kmalloc: corrupted chunk");
#endif
	assert(c->used, "kmalloc: pointer was not allocated");
}
