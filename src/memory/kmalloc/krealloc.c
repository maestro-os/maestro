#include <memory/kmalloc/kmalloc.h>
#include <memory/kmalloc/kmalloc_internal.h>

/*
 * Shrinks the given `chunk` to the specified `size`. `size` must be lower than
 * the size of the chunk. Shrinking the chunk might create a new unused chunk
 * that might be merged with the following chunk.
 */
static void shrink_chunk(used_chunk_t *chunk, const size_t size)
{
	chunk_hdr_t *next;

	split_chunk(&chunk->hdr, size);
	next = chunk->hdr.next;
	if(!next || !next->next)
		return;
	if(next->used || next->next->used)
		return;
	merge_chunks(next);
}

/*
 * Increases the size of the chunk by eating the next chunk. The chunk will take
 * the size `size`. Next chunk will shrink if enough space is remaining, else
 * it might disapear.
 */
static void eat_chunk(used_chunk_t *chunk, const size_t size)
{
	merge_chunks(&chunk->hdr);
	split_chunk(&chunk->hdr, size);
}

/*
 * If `ptr` is `NULL` and `size` is not zero, the function is equivalent to
 * `malloc`. If `size` is zero and `ptr` is not NULL, the function is equivalent
 * to free.
 *
 * Else, the function tries to increase the given chunk of memory to the given
 * new size. If the size of the chunk cannot be increased, a new chunk will be
 * allocated, the data from the old chunk will be copied to the new one and
 * the old one will be freed.
 */
ATTR_MALLOC
void *krealloc(void *ptr, const size_t size)
{
	chunk_hdr_t *c;
	void *p;

	if(!ptr)
		return kmalloc(size);
	if(size == 0)
	{
		kfree(ptr);
		return NULL;
	}
	spin_lock(&kmalloc_spinlock);
	c = GET_CHUNK(ptr);
	chunk_assert(c);
	if(size <= c->size)
	{
		shrink_chunk((used_chunk_t *) c, size);
		return ptr;
	}
	if(c->next && !c->next->used
		&& (c->next->size + CHUNK_HDR_SIZE) - c->size >= size)
	{
		eat_chunk((used_chunk_t *) c, size);
		spin_unlock(&kmalloc_spinlock);
		return ptr;
	}
	if(!(p = kmalloc(size)))
		return NULL;
	spin_unlock(&kmalloc_spinlock);
	memcpy(p, ptr, MIN(c->size, size));
	kfree(ptr);
	return p;
}
