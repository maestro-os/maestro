#include <kernel.h>
#include <memory/kmalloc/kmalloc_internal.h>
#include <memory/buddy/buddy.h>
#include <debug/debug.h>

/*
 * This file handles internal operations for kmalloc.
 *
 * Allocations are stored into a linked list of chunks. Every chunk is allocated
 * according to the value into `ALIGNMENT`.
 *
 * Chunks are contained into blocks which represent the memory regions given by
 * the buddy allocator.
 *
 * MALLOC_CHUNK_MAGIC is a magic number used in chunk structures to ensure that
 * chunks aren't corrupted. If the value has been changed between two
 * operations of the allocator, the kernel shall panic.
 */

/*
 * Bins containing the list of allocated blocks.
 */
static block_t *blocks_bin = NULL;

/*
 * Bucket containing the list of free chunks.
 * Lists are sorted according to the size of the empty chunk.
 */
ATTR_BSS
static free_chunk_t *buckets[BUCKETS_COUNT];

/*
 * The spinlock for kmalloc operations.
 */
spinlock_t kmalloc_spinlock;

/*
 * Links the given block to the given bin.
 */
static inline void bin_link(block_t *block)
{
	debug_assert(sanity_check(block), "bin_link: bad argument");
	if((block->next = blocks_bin))
		block->next->prev = block;
	blocks_bin = block;
}

/*
 * Allocates a `pages` pages long block of memory and creates a chunk on it that
 * covers the whole block.
 */
ATTR_MALLOC
block_t *kmalloc_alloc_block(const size_t pages)
{
	size_t buddy_order;
	block_t *b;
	chunk_hdr_t *first_chunk;

	if(pages == 0)
		return NULL;
	buddy_order = buddy_get_order(pages);
	if(!(b = buddy_alloc_zero(buddy_order)))
		return NULL;
	bzero(b, BLOCK_HDR_SIZE);
	b->buddy_order = buddy_order;
	first_chunk = BLOCK_DATA(b);
	first_chunk->block = b;
	first_chunk->size = FRAME_SIZE(buddy_order)
		- (BLOCK_HDR_SIZE + CHUNK_HDR_SIZE);
#ifdef MALLOC_CHUNK_MAGIC
	first_chunk->magic = MALLOC_CHUNK_MAGIC;
#endif
	bin_link(b);
	return b;
}

/*
 * Unlinks the given block `b` from its bin and frees it.
 */
void kmalloc_free_block(block_t *b)
{
	if(!sanity_check(b))
		return;
	if(b->prev)
		b->prev->next = b->next;
	else
		blocks_bin = b->next;
	if(b->next)
		b->next->prev = b->prev;
	buddy_free(b, buddy_get_order(b->buddy_order));
}

/*
 * Returns a small bucket containing chunks large enough to fit an allocation of
 * the given `size`. If `insert` is not zero, the function will return the first
 * bucket that fits even if empty to allow insertion of a new free chunk.
 */
free_chunk_t **get_bucket(const size_t size, const int insert)
{
	size_t i = 0;

	if(size < FIRST_BUCKET_SIZE)
		return NULL;
	if(insert)
	{
		while(size >= (FIRST_BUCKET_SIZE << (i + 1)) && i < BUCKETS_COUNT - 1)
			++i;
	}
	else
	{
		while((!buckets[i] || size > (FIRST_BUCKET_SIZE << i))
			&& i < BUCKETS_COUNT - 1)
			++i;
	}
	return &buckets[i];
}

/*
 * Links the given free chunk to the corresponding bucket.
 */
void bucket_link(free_chunk_t *chunk)
{
	free_chunk_t **bucket;

	debug_assert(sanity_check(chunk), "bucket_link: bad argument");
	if(!(bucket = get_bucket(chunk->hdr.size, 1)))
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

	debug_assert(sanity_check(chunk), "bucket_unlink: bad argument");
	// TODO Check block type instead of checking both small and medium?
	for(i = 0; i < BUCKETS_COUNT; ++i)
		if(buckets[i] == chunk)
			buckets[i] = chunk->next_free;
	if(sanity_check(chunk->prev_free))
		chunk->prev_free->next_free = chunk->next_free;
	if(sanity_check(chunk->next_free))
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

	debug_assert(sanity_check(chunk) && size > 0, "split_chunk: bad arguments");
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
	if((new_chunk->hdr.next = (chunk_hdr_t *) sanity_check(chunk->next)))
		new_chunk->hdr.next->prev = (chunk_hdr_t *) new_chunk;
	if((new_chunk->hdr.prev = (chunk_hdr_t *) chunk))
		new_chunk->hdr.prev->next = (chunk_hdr_t *) new_chunk;
	new_chunk->hdr.block = sanity_check(chunk->block);
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
	debug_assert(sanity_check(chunk) && size > 0, "alloc_chunk: bad arguments");
#ifdef MALLOC_CHUNK_MAGIC
	debug_assert(chunk->hdr.magic == MALLOC_CHUNK_MAGIC,
		"kmalloc: corrupted chunk");
#endif
	debug_assert(!chunk->hdr.used && chunk->hdr.size >= size,
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
	debug_assert(sanity_check(c), "alloc_chunk: bad argument");
#ifdef MALLOC_CHUNK_MAGIC
	debug_assert(c->magic == MALLOC_CHUNK_MAGIC, "kmalloc: corrupted chunk");
#endif
	debug_assert(c->used, "kmalloc: pointer was not allocated");
#ifndef KERNEL_DEBUG
	(void) c;
#endif
}
