#include <memory/kmalloc/kmalloc.h>
#include <libc/errno.h>

__attribute__((hot))
void *kmalloc(const size_t size, const int flags)
{
	chunk_t *chunk;

	errno = 0;
	if(size == 0)
		return NULL;
	if(!(chunk = get_free_chunk(size, flags)))
	{
		errno = ENOMEM;
		return NULL;
	}
	alloc_chunk(chunk, size);
	return CHUNK_CONTENT(chunk);
}

__attribute__((hot))
void *kmalloc_zero(const size_t size, const int flags)
{
	void *ptr;

	if((ptr = kmalloc(size, flags)))
		bzero(ptr, size);
	return ptr;
}
