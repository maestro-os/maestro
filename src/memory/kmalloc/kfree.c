#include <memory/kmalloc/kmalloc.h>
#include <memory/kmalloc/kmalloc_internal.h>

/*
 * Frees the given memory chunk.
 * Does nothing if `ptr` is `NULL`.
 *
 * The function shall make the current process abort
 * if the given `ptr` is invalid.
 */
void kfree(void *ptr)
{
	chunk_hdr_t *c;

	if(unlikely(!ptr))
		return;
	spin_lock(&kmalloc_spinlock);
	c = GET_CHUNK(ptr);
	chunk_assert(c);
	c->used = 0;
	bucket_link((free_chunk_t *) c);
	if(c->next && !c->next->used)
		merge_chunks(c);
	if(c->prev && !c->prev->used)
	{
		c = c->prev;
		merge_chunks(c);
	}
	if(c->prev || c->next)
	{
		spin_unlock(&kmalloc_spinlock);
		return;
	}
	bucket_unlink((free_chunk_t *) c);
	kmalloc_free_block(c->block);
	spin_unlock(&kmalloc_spinlock);
}
