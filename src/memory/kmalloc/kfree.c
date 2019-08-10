#include <memory/kmalloc/kmalloc.h>

void kfree(void *ptr, const int flags)
{
	chunk_t *chunk;

	if(!ptr || !(chunk = get_chunk(ptr)))
		return;
	free_chunk(chunk, flags);
}
