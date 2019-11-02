#include <memory/kmalloc/kmalloc.h>
#include <libc/errno.h>
#include <util/attr.h>

#include <libc/stdio.h> // TODO remove

// TODO Fix buckets (probably inefficient currently)

spinlock_t kmalloc_spinlock = 0;

__ATTR_BSS
static chunk_t *buckets[BUCKETS_COUNT];
__ATTR_BSS
static chunk_t *large_chunks;

chunk_t *get_chunk(void *ptr)
{
	size_t i;
	chunk_t *c;

	if(!ptr || ptr < buddy_begin)
		return NULL;
	// TODO Optimize
	for(i = 0; i < BUCKETS_COUNT; ++i)
	{
		c = buckets[i];
		while(c)
		{
			if(CHUNK_CONTENT(c) == ptr)
				return c;
			c = c->next;
		}
	}
	return NULL;
}

static void coalesce_chunks(chunk_t *chunk)
{
	if(CHUNK_IS_USED(chunk))
		return;
	if(chunk->next && !CHUNK_IS_USED(chunk->next)
		&& SAME_PAGE(chunk, chunk->next))
	{
		chunk->size += chunk->next->size;
		if((chunk->next = chunk->next->next))
			chunk->next->prev = chunk;
	}
	if(chunk->prev && !CHUNK_IS_USED(chunk->prev)
		&& SAME_PAGE(chunk, chunk->prev))
		coalesce_chunks(chunk->prev);
}

static void *_alloc_block(const size_t pages, const int flags)
{
	if(flags & KMALLOC_BUDDY)
		return buddy_alloc_zero(buddy_get_order(pages * PAGE_SIZE));
	else
		return pages_alloc_zero(pages);
}

static chunk_t *alloc_block(chunk_t **bucket, const int flags)
{
	chunk_t *ptr;

	if(!(ptr = _alloc_block(1, flags)))
		return NULL;
	ptr->size = PAGE_SIZE - sizeof(chunk_t);
	if(flags & KMALLOC_BUDDY)
		ptr->flags |= CHUNK_FLAG_BUDDY;
	if((ptr->next = *bucket))
		ptr->next->prev = ptr;
	*bucket = ptr;
	return ptr;
}

static void *large_alloc(const size_t size, const int flags)
{
	size_t total_size, pages;
	chunk_t *chunk;

	total_size = sizeof(chunk_t) + size;
	pages = UPPER_DIVISION(total_size, PAGE_SIZE);
	if((chunk = _alloc_block(pages, flags)))
	{
		chunk->size = pages * PAGE_SIZE + sizeof(chunk_t);
		if(flags & KMALLOC_BUDDY)
			chunk->flags |= CHUNK_FLAG_BUDDY;
		if((chunk->next = large_chunks))
			chunk->next->prev = chunk;
		large_chunks = chunk;
	}
	return chunk;
}

static chunk_t *bucket_get_free_chunk(chunk_t **bucket, const size_t size,
	const int flags)
{
	chunk_t *c;

	c = *bucket;
	if(flags & KMALLOC_BUDDY)
	{
		while(c && (CHUNK_IS_USED(c) || !(c->flags & CHUNK_FLAG_BUDDY)
			|| c->size < size))
			c = c->next;
	}
	else
		while(c && (CHUNK_IS_USED(c) || (c->flags & CHUNK_FLAG_BUDDY)
			|| c->size < size))
			c = c->next;
	return c;
}

chunk_t *get_free_chunk(const size_t size, const int flags)
{
	size_t i = 0;
	chunk_t **bucket, *c;

	while(SMALLER_BUCKET * POW2(i) < size)
		++i;
	if(i < BUCKETS_COUNT)
		bucket = buckets + i;
	else
		return large_alloc(size, flags);
	if(!(c = bucket_get_free_chunk(bucket, size, flags)))
		c = alloc_block(bucket, flags);
	return c;
}

void alloc_chunk(chunk_t *chunk, const size_t size)
{
	chunk_t *next;

	if(!chunk)
		return;
	if(chunk->size + sizeof(chunk_t) > size)
	{
		next = (void *) chunk + sizeof(chunk_t) + size;
		next->prev = chunk;
		if((next->next = chunk->next))
			next->next->prev = next;
		next->size = chunk->size - size - sizeof(chunk_t);
		next->flags = chunk->flags & ~CHUNK_FLAG_USED;
		chunk->next = next;
	}
	chunk->flags |= CHUNK_FLAG_USED;
}

void free_chunk(chunk_t *chunk, const int flags)
{
	if(!chunk)
		return;
	chunk->flags &= ~CHUNK_FLAG_USED;
	coalesce_chunks(chunk);
	// TODO If page is empty, free it
	(void) flags;
}
