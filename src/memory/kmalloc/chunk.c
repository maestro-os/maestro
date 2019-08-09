#include <memory/kmalloc/kmalloc.h>
#include <libc/errno.h>

__attribute__((section("bss")))
static chunk_t *buckets[BUCKETS_COUNT];
// TODO
/*__attribute__((section("bss")))
static chunk_t *large_chunks;*/

chunk_t *get_chunk(void *ptr)
{
	if(!ptr)
		return NULL;
	// TODO
	return NULL;
}

static void alloc_block(chunk_t **bucket)
{
	chunk_t *ptr;

	if(!(ptr = buddy_alloc(0))) // TODO Alloc multiple pages if needed?
		return;
	ptr->next = *bucket;
	ptr->size = PAGE_SIZE - sizeof(chunk_t);
	*bucket = ptr;
}

chunk_t *get_free_chunk(const size_t size)
{
	size_t i = 0;
	chunk_t *c;

	while(SMALLER_BUCKET * POW2(i) < size)
		++i;
	if(i >= BUCKETS_COUNT)
	{
		// TODO Large alloc
		return NULL;
	}
	c = buckets[i];
	while(c && (c->used || c->size < size))
		c = c->next;
	if(!c)
	{
		alloc_block(&buckets[i]);
		if(!errno)
		{
			c = buckets[i];
			while(c && (c->used || c->size < size))
				c = c->next;
		}
	}
	return c;
}

void alloc_chunk(chunk_t *chunk)
{
	if(!chunk)
		return;
	// TODO
	(void) chunk;
}

void free_chunk(chunk_t *chunk)
{
	if(!chunk)
		return;
	// TODO
	(void) chunk;
}
