#include <memory/kmalloc/kmalloc.h>
#include <memory/kmalloc/kmalloc_internal.h>
#include <memory/memory.h>
#include <libc/errno.h>

/*
 * Handles a small allocation.
 */
ATTR_MALLOC
void *small_alloc(const size_t size)
{
	free_chunk_t **bucket;
	block_t *b;
	free_chunk_t *chunk;

	if(!(bucket = get_bucket(size, 0, 0)) || !*bucket)
	{
		if(!(b = kmalloc_alloc_block(SMALL_BLOCK_PAGES)))
			return NULL;
		chunk = BLOCK_DATA(b);
	}
	else
		chunk = *bucket;
	alloc_chunk(chunk, size);
	return CHUNK_DATA(chunk);
}

/*
 * Handles a medium allocation.
 */
ATTR_MALLOC
void *medium_alloc(const size_t size)
{
	free_chunk_t **bucket;
	block_t *b;
	free_chunk_t *chunk;

	if(!(bucket = get_bucket(size, 0, 1)) || !*bucket)
	{
		if(!(b = kmalloc_alloc_block(MEDIUM_BLOCK_PAGES)))
			return NULL;
		chunk = BLOCK_DATA(b);
	}
	else
		chunk = *bucket;
	alloc_chunk(chunk, size);
	return CHUNK_DATA(chunk);
}

/*
 * Handles a large allocation.
 */
ATTR_MALLOC
void *large_alloc(const size_t size)
{
	size_t min_size;
	block_t *b;
	chunk_hdr_t *first_chunk;

	min_size = BLOCK_HDR_SIZE + CHUNK_HDR_SIZE + size;
	if(!(b = kmalloc_alloc_block(CEIL_DIVISION(min_size, PAGE_SIZE))))
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
ATTR_MALLOC
void *kmalloc(const size_t size)
{
	void *ptr;

	if(size == 0)
		return NULL;
	spin_lock(&kmalloc_spinlock);
	if(size < SMALL_BIN_MAX)
		ptr = small_alloc(size);
	else if(size < MEDIUM_BIN_MAX)
		ptr = medium_alloc(size);
	else
		ptr = large_alloc(size);
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
