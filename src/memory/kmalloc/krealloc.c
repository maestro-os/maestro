#include <memory/kmalloc/kmalloc.h>
#include <memory/kmalloc/kmalloc_internal.h>
#include <util/util.h>

/*
 * Moves the given chunk somewhere else. Returns the pointer to the new location
 * of the chunk.
 */
static void *move_chunk(kmalloc_used_chunk_t *chunk, size_t new_size)
{
	void *new;

	debug_assert(sanity_check(chunk) && new_size > 0,
		"kmalloc: invalid arguments");
	if(!(new = kmalloc(new_size)))
		return NULL;
	memcpy(new, chunk->data, MIN(chunk->hdr.size, new_size));
	kfree(chunk->data);
	return new;
}

/*
 * Changes the size of the given memory region that was allocated with
 * `kmalloc`. Passing a NULL pointer is equivalent to calling `kmalloc` with the
 * specified `size`. If `ptr` is not NULL but `size` equals `0`, it is
 * equivalent to calling `kfree` with `ptr`.
 *
 * If the size of the region is increased, the newly allocated region is not
 * allocated.
 * If the region has to be moved, the data is copied to a new region of memory,
 * the old region is freed and the pointer to the beginning of the new region is
 * returned.
 */
void *krealloc(void *ptr, size_t size)
{
	kmalloc_used_chunk_t *chunk;
	kmalloc_chunk_hdr_t *next;

	if(!sanity_check(ptr))
		return kmalloc(size);
	if(size == 0)
	{
		kfree(ptr);
		return NULL;
	}
	spin_lock(&kmalloc_spinlock);
	chunk = CONTAINER_OF(ptr, kmalloc_used_chunk_t, data);
	if(chunk->hdr.size > size)
		consume_chunk(&chunk->hdr, size);
	else if(chunk->hdr.size < size)
	{
		next = CONTAINER_OF(chunk->hdr.list.next, kmalloc_chunk_hdr_t, list);
		if(next && (next->flags & KMALLOC_FLAG_USED)
			&& sizeof(kmalloc_chunk_hdr_t) + next->size >= size)
		{
			// TODO If followed by a free chunk large enough, eat/shrink it
		}
		else
			ptr = move_chunk(chunk, size);
	}
	spin_unlock(&kmalloc_spinlock);
	return ptr;
}
