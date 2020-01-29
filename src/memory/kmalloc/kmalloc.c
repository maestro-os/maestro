#include <memory/kmalloc/kmalloc.h>
#include <memory/kmalloc/kmalloc_internal.h>
#include <memory/memory.h>
#include <libc/errno.h>

/*
 * Handles a small allocation.
 */
__attribute__((malloc))
void *_small_alloc(const size_t size)
{
	_free_chunk_t **bucket;
	_block_t *b;
	_free_chunk_t *chunk;

	if(!(bucket = _get_bucket(size, 0, 0)) || !*bucket)
	{
		if(!(b = _alloc_block(_SMALL_BLOCK_PAGES)))
			return NULL;
		chunk = BLOCK_DATA(b);
	}
	else
		chunk = *bucket;
	_alloc_chunk(chunk, size);
	return CHUNK_DATA(chunk);
}

/*
 * Handles a medium allocation.
 */
__attribute__((malloc))
void *_medium_alloc(const size_t size)
{
	_free_chunk_t **bucket;
	_block_t *b;
	_free_chunk_t *chunk;

	if(!(bucket = _get_bucket(size, 0, 1)) || !*bucket)
	{
		if(!(b = _alloc_block(_MEDIUM_BLOCK_PAGES)))
			return NULL;
		chunk = BLOCK_DATA(b);
	}
	else
		chunk = *bucket;
	_alloc_chunk(chunk, size);
	return CHUNK_DATA(chunk);
}

/*
 * Handles a large allocation.
 */
__attribute__((malloc))
void *_large_alloc(const size_t size)
{
	size_t min_size;
	_block_t *b;
	_chunk_hdr_t *first_chunk;

	min_size = BLOCK_HDR_SIZE + CHUNK_HDR_SIZE + size;
	if(!(b = _alloc_block(CEIL_DIVISION(min_size, PAGE_SIZE))))
		return NULL;
	first_chunk = BLOCK_DATA(b);
	first_chunk->used = 1;
	return CHUNK_DATA(first_chunk);
}

/*
 * Allocates a chunk of memory of size `size` and returns a pointer to the
 * beginning. Pointer is suitably aligned to fit any built-in type.
 *
 * Memory chunk is cleared before being returned.
 * If a size of zero is given, `NULL` is returned.
 *
 * If the allocation fails, the errno is set to ENOMEM.
 */
__attribute__((malloc))
void *kmalloc(const size_t size)
{
	void *ptr;

	if(size == 0)
		return NULL;
	spin_lock(&kmalloc_spinlock);
	if(size < _SMALL_BIN_MAX)
		ptr = _small_alloc(size);
	else if(size < _MEDIUM_BIN_MAX)
		ptr = _medium_alloc(size);
	else
		ptr = _large_alloc(size);
	if(!ptr)
		errno = ENOMEM;
	spin_unlock(&kmalloc_spinlock);
	return ptr;
}

/*
 * Allocates a chunk of memory using `kmalloc` and initializes the memory to
 * zero.
 */
void *kmalloc_zero(const size_t size)
{
	void *ptr;

	if((ptr = kmalloc(size)))
		bzero(ptr, size);
	return ptr;
}
