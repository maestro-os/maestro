#include <memory/memory.h>

void kfree(void *ptr)
{
	chunk_t *chunk;

	if(!ptr || !(chunk = get_chunk(ptr)))
		return;
	free_chunk(chunk);
}
