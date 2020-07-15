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
	debug_assert(sanity_check(chunk), "kmalloc: invalid argument");
	eat_next(&chunk->list);
	if(eat_next(chunk->list.prev))
		chunk = CONTAINER_OF(chunk->list.prev, kmalloc_chunk_hdr_t, list);
	return chunk;
}

/*
 * Frees the given memory region allocated with `kmalloc`.
 */
void kfree(void *ptr)
{
	kmalloc_chunk_hdr_t *chunk;
	kmalloc_block_t *block;

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
		block = CONTAINER_OF(chunk, kmalloc_block_t, data);
		debug_assert(IS_ALIGNED(block, PAGE_SIZE), "kmalloc: invalid block");
		buddy_free(block, block->buddy_order);
	}
	else
		free_bin_insert((kmalloc_free_chunk_t *) chunk);
	spin_unlock(&kmalloc_spinlock);
}
