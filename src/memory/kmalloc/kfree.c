#include <kernel.h>
#include <memory/kmalloc/kmalloc.h>

void kfree(void *ptr, const int flags)
{
	chunk_t *chunk;

	if(!ptr)
		return;
	if(ptr < buddy_get_begin() || !(chunk = get_chunk(ptr)))
		PANIC("Invalid kfree!", 0);
	free_chunk(chunk, flags);
}
