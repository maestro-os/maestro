#include <memory/kmalloc/kmalloc.h>
#include <libc/errno.h>

spinlock_t kmalloc_spinlock = 0;

__attribute__((section("bss")))
static chunk_t *buckets[BUCKETS_COUNT];
__attribute__((section("bss")))
static chunk_t *large_chunks;

chunk_t *get_chunk(void *ptr)
{
	size_t i;
	chunk_t *c;

	if(!ptr)
		return NULL;
	// TODO Optimize
	for(i = 0; i < BUCKETS_COUNT; ++i)
	{
		if(!buckets[i])
			continue;
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
	chunk_t *begin, *end;
	size_t size = 0;

	if(CHUNK_IS_USED(chunk))
		return;
	begin = chunk;
	end = chunk;
	while(begin->prev && !CHUNK_IS_USED(begin->prev)
		&& SAME_PAGE(begin, begin->prev))
		begin = begin->prev;
	while(end && !CHUNK_IS_USED(end) && SAME_PAGE(end, end->prev))
		end = end->next;
	if(begin == end)
		return;
	chunk = begin;
	while(chunk != end)
	{
		size += chunk->size;
		if(chunk->next != end)
			size += sizeof(chunk_t);
		chunk = chunk->next;
	}
	begin->size = size;
	begin->next = end;
}

static void *_alloc_block(const size_t pages, const int flags)
{
	if(flags & KMALLOC_BUDDY)
		return buddy_alloc_zero(buddy_get_order(pages * PAGE_SIZE));
	else
		return pages_alloc_zero(pages);
}

static void alloc_block(chunk_t **bucket, const int flags)
{
	chunk_t *ptr;

	if(!(ptr = _alloc_block(1, flags)))
		return;
	ptr->size = PAGE_SIZE - sizeof(chunk_t);
	if(flags & KMALLOC_BUDDY)
		ptr->flags |= CHUNK_FLAG_BUDDY;
	if((ptr->next = *bucket))
		ptr->next->prev = ptr;
	*bucket = ptr;
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
		chunk->next = large_chunks;
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
	{
		alloc_block(bucket, flags);
		c = bucket_get_free_chunk(bucket, size, flags);
	}
	return c;
}

void alloc_chunk(chunk_t *chunk, const size_t size)
{
	chunk_t *next;

	if(!chunk)
		return;
	if(chunk->size + sizeof(chunk_t) > size)
	{
		next = (void *) chunk + size;
		next->prev = chunk;
		next->next = chunk->next;
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
	if(chunk->prev)
		chunk->prev->next = chunk->next;
	if(chunk->next)
		chunk->next->prev = chunk->prev;
	chunk->flags &= ~CHUNK_FLAG_USED;
	coalesce_chunks(chunk);
	// TODO If page is empty, free it
	(void) flags;
}
