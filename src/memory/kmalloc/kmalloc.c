#include <memory/kmalloc/kmalloc.h>
#include <memory/kmalloc/kmalloc_internal.h>
#include <memory/memory.h>
#include <libc/errno.h>

/*
 * Handles an allocation. This function might allocate a new block of memory to
 * fullfill the operation.
 */
ATTR_MALLOC
void *alloc_(const size_t size)
{
	free_chunk_t **bucket;
	size_t pages;
	block_t *b;
	free_chunk_t *chunk;

	if(!(bucket = get_bucket(size, 0)) || !*bucket)
	{
		pages = CEIL_DIVISION(sizeof(block_t) + size, PAGE_SIZE);
		if(!(b = kmalloc_alloc_block(pages)))
			return NULL;
		chunk = BLOCK_DATA(b);
	}
	else
		chunk = *bucket;
	alloc_chunk(chunk, size);
	return CHUNK_DATA(chunk);
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
	ptr = alloc_(size);
	spin_unlock(&kmalloc_spinlock);
	if(!ptr)
		errno = ENOMEM;
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
