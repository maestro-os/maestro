#include <memory/kmalloc/kmalloc.h>
#include <libc/errno.h>

__attribute__((hot))
void *kmalloc(const size_t size)
{
	chunk_t *chunk;

	errno = 0;
	if(size == 0)
		return NULL;
	if(!(chunk = get_free_chunk(size)))
	{
		errno = ENOMEM;
		return NULL;
	}
	alloc_chunk(chunk);
	return CHUNK_CONTENT(chunk);
}

__attribute__((hot))
void *kmalloc_zero(const size_t size)
{
	void *ptr;

	if((ptr = kmalloc(size)))
		bzero(ptr, size);
	return ptr;
}
