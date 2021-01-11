#include <memory/kmalloc/kmalloc.h>
#include <memory/kmalloc/kmalloc_internal.h>
#include <kernel.h>
#include <debug/debug.h>

/*
 * Merges the given `chunk` with the next if it is not used.
 * Returns `0` if nothing was done, else `1`.
 */
static int eat_next(list_head_t *chunk)
{
	list_head_t *l_next;
	kmalloc_chunk_hdr_t *curr, *next;

	debug_assert(sanity_check(chunk), "kmalloc: invalid argument");
	if((l_next = chunk->next))
	{
		curr = CONTAINER_OF(chunk, kmalloc_chunk_hdr_t, list);
		if(curr->flags & KMALLOC_FLAG_USED)
			return 0;
		next = CONTAINER_OF(l_next, kmalloc_chunk_hdr_t, list);
		if(!(next->flags & KMALLOC_FLAG_USED))
		{
			free_bin_remove((kmalloc_free_chunk_t *) next);
			curr->size += sizeof(kmalloc_chunk_hdr_t) + next->size;
			list_remove(NULL, l_next);
			return 1;
		}
	}
	return 0;
}

/*
 * Coalesces chunks adjacent to the given one.
 */
static kmalloc_chunk_hdr_t *coalesce_adjacents(kmalloc_chunk_hdr_t *chunk)
{
	list_head_t *prev;

	debug_assert(sanity_check(chunk), "kmalloc: invalid argument");
	debug_assert(!chunk->list.prev
		|| (void *) chunk->list.prev < (void *) chunk,
		"kmalloc: invalid previous chunk");
	debug_assert(!chunk->list.next
		|| (void *) chunk->list.next > (void *) chunk,
		"kmalloc: invalid next chunk");
	prev = chunk->list.prev;
	eat_next(&chunk->list);
	if(prev && eat_next(prev))
		chunk = CONTAINER_OF(prev, kmalloc_chunk_hdr_t, list);
	return chunk;
}

/*
 * Frees the given memory region allocated with `kmalloc`.
 */
void kfree(void *ptr)
{
	kmalloc_chunk_hdr_t *chunk;

	if(!sanity_check(ptr))
		return;
	spin_lock(&kmalloc_spinlock);
	chunk = &CONTAINER_OF(ptr, kmalloc_used_chunk_t, data)->hdr;
#ifdef KMALLOC_MAGIC
	check_magic(chunk);
#endif
	if(!(chunk->flags & KMALLOC_FLAG_USED))
		PANIC("kmalloc: trying to free unused chunk", 0);
	chunk->flags &= ~KMALLOC_FLAG_USED;
	bzero(&((kmalloc_free_chunk_t *) chunk)->free_list, sizeof(list_head_t));
	chunk = coalesce_adjacents(chunk);
	if(!chunk->list.prev && !chunk->list.next)
	{
		debug_assert(chunk->block == CONTAINER_OF(chunk, kmalloc_block_t, data),
			"kmalloc: bad block address");
		check_block(chunk->block);
		buddy_free(chunk->block, chunk->block->buddy_order);
	}
	else
		free_bin_insert((kmalloc_free_chunk_t *) chunk);
	spin_unlock(&kmalloc_spinlock);
}
