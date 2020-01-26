#include <kernel.h>
#include <memory/kmalloc/kmalloc_internal.h>

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
 * - _small_bin: size < _SMALL_BIN_MAX
 * - _medium_bin: size < _MEDIUM_BIN_MAX
 * - _large_bin: size >= _MEDIUM_BIN_MAX
 *
 * Blocks in `_large_bin` contain only one allocation which can be several
 * pages large.
 *
 * _MALLOC_CHUNK_MAGIC is a magic number used in chunk structures to ensure that
 * chunks aren't overwritten. If the value has been changed between two
 * operations of the allocator, the current process shall abort.
 */

/*
 * Bins containing the list of allocated blocks.
 */
_block_t *_small_bin = NULL;
_block_t *_medium_bin = NULL;
_block_t *_large_bin = NULL;

/*
 * Buckets containing lists of free chunks.
 * Lists are sorted according to the size of the empty chunk.
 *
 * A chunk must be at least `n` bytes large to fit in a bucket, where
 * n=_FIRST_SMALL_BUCKET_SIZE * 2^i . Here, `i` is the index in the array.
 */
__attribute__((section(".bss")))
_free_chunk_t *_small_buckets[_SMALL_BUCKETS_COUNT];
__attribute__((section(".bss")))
_free_chunk_t *_medium_buckets[_MEDIUM_BUCKETS_COUNT];

/*
 * Links the given block to the given bin.
 */
static inline void _bin_link(_block_t *block)
{
	_block_t **bin;

	bin = _block_get_bin(block);
	if((block->next = *bin))
		block->next->prev = block;
	*bin = block;
}

/*
 * Allocates a `pages` pages long block of memory and creates a chunk on it that
 * covers the whole block.
 */
__attribute__((malloc))
_block_t *_alloc_block(const size_t pages)
{
	_block_t *b;
	_chunk_hdr_t *first_chunk;

	if(pages == 0 || !(b = pages_alloc(pages)))
		return NULL;
	bzero(b, BLOCK_HDR_SIZE);
	b->pages = pages;
	first_chunk = BLOCK_DATA(b);
	first_chunk->block = b;
	first_chunk->size = pages * PAGE_SIZE - (BLOCK_HDR_SIZE + CHUNK_HDR_SIZE);
#ifdef _MALLOC_CHUNK_MAGIC
	first_chunk->magic = _MALLOC_CHUNK_MAGIC;
#endif
	_bin_link(b);
	return b;
}

/*
 * Returns a pointer to the bin for the given block.
 */
_block_t **_block_get_bin(_block_t *b)
{
	if(b->pages <= _SMALL_BLOCK_PAGES)
		return &_small_bin;
	else if(b->pages <= _MEDIUM_BLOCK_PAGES)
		return &_medium_bin;
	return &_large_bin;
}

/*
 * Unlinks the given block `b` from its bin and frees it.
 */
void _free_block(_block_t *b)
{
	if(b->prev)
		b->prev->next = b->next;
	else if(b == _small_bin)
		_small_bin = b->next;
	else if(b == _medium_bin)
		_medium_bin = b->next;
	else if(b == _large_bin)
		_large_bin = b->next;
	if(b->next)
		b->next->prev = b->prev;
	pages_free(b);
}

/*
 * Returns a small bucket containing chunks large enough to fit an allocation of
 * the given `size`. If `insert` is not zero, the function will return the first
 * bucket that fits even if empty to allow insertion of a new free chunk.
 */
_free_chunk_t **_get_bucket(const size_t size, const int insert,
	const int medium)
{
	_free_chunk_t **buckets;
	size_t first, count;
	size_t i = 0;

	if(medium)
	{
		buckets = _medium_buckets;
		first = _FIRST_MEDIUM_BUCKET_SIZE;
		count = _MEDIUM_BUCKETS_COUNT;
	}
	else
	{
		buckets = _small_buckets;
		first = _FIRST_SMALL_BUCKET_SIZE;
		count = _SMALL_BUCKETS_COUNT;
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
void _bucket_link(_free_chunk_t *chunk)
{
	_free_chunk_t **bucket;

	if(!(bucket = _get_bucket(chunk->hdr.size, 1,
		_block_get_bin(chunk->hdr.block) == &_medium_bin)))
		return;
	chunk->prev_free = NULL;
	if((chunk->next_free = *bucket))
		chunk->next_free->prev_free = chunk;
	*bucket = chunk;
}

/*
 * Unlinks the given free chunk from its bucket.
 */
void _bucket_unlink(_free_chunk_t *chunk)
{
	size_t i;

	// TODO Check block type instead of checking both small and medium?
	for(i = 0; i < _SMALL_BUCKETS_COUNT; ++i)
		if(_small_buckets[i] == chunk)
			_small_buckets[i] = chunk->next_free;
	for(i = 0; i < _MEDIUM_BUCKETS_COUNT; ++i)
		if(_medium_buckets[i] == chunk)
			_medium_buckets[i] = chunk->next_free;
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
void _split_chunk(_chunk_hdr_t *chunk, size_t size)
{
	_free_chunk_t *new_chunk;
	size_t l;

	size = MAX(ALIGNMENT, size);
	new_chunk = (_free_chunk_t *) ALIGN(CHUNK_DATA(chunk) + size, ALIGNMENT);
	if(chunk->size <= size + CHUNK_HDR_SIZE + ALIGNMENT)
		return;
	l = chunk->size;
	chunk->size = size;
	if(!chunk->used)
	{
		_bucket_unlink((_free_chunk_t *) chunk);
		_bucket_link((_free_chunk_t *) chunk);
	}
	if((new_chunk->hdr.next = (_chunk_hdr_t *) chunk->next))
		new_chunk->hdr.next->prev = (_chunk_hdr_t *) new_chunk;
	if((new_chunk->hdr.prev = (_chunk_hdr_t *) chunk))
		new_chunk->hdr.prev->next = (_chunk_hdr_t *) new_chunk;
	new_chunk->hdr.block = chunk->block;
	new_chunk->hdr.size = l - (size + CHUNK_HDR_SIZE);
	new_chunk->hdr.used = 0;
#ifdef _MALLOC_CHUNK_MAGIC
	new_chunk->hdr.magic = _MALLOC_CHUNK_MAGIC;
#endif
	_bucket_link(new_chunk);
}

/*
 * Merges the given chunk with the following chunk.
 */
void _merge_chunks(_chunk_hdr_t *c)
{
	if(!c->next->used)
		_bucket_unlink((_free_chunk_t *) c->next);
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
void _alloc_chunk(_free_chunk_t *chunk, const size_t size)
{
#ifdef _MALLOC_CHUNK_MAGIC
	if(unlikely(chunk->hdr.magic != _MALLOC_CHUNK_MAGIC))
		PANIC("kmalloc: corrupted chunk", 0);
#endif
	if(unlikely(chunk->hdr.used || chunk->hdr.size < size))
		PANIC("kmalloc: internal error", 0);
	_bucket_unlink(chunk);
	chunk->hdr.used = 1;
	_split_chunk(&chunk->hdr, size);
}

/*
 * Checks that the given chunk is valid for reallocation/freeing.
 * If the chunk is invalid, prints an error message and aborts.
 */
void _chunk_assert(_chunk_hdr_t *c)
{
#ifdef _MALLOC_CHUNK_MAGIC
	if(unlikely(c->magic != _MALLOC_CHUNK_MAGIC))
		PANIC("kmalloc: corrupted chunk", 0);
#endif
	if(unlikely(!c->used))
		PANIC("kmalloc: pointer was not allocated", 0);
}
