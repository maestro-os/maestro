#include "memory.h"
#include "kalloc_internal.h"
#include "../libc/errno.h"

static mem_chunk_t *new_chunk(mem_chunk_t *chunks)
{
	mem_chunk_t *c = chunks;
	if(c)
		while(c->next) c = c->next;

	mem_chunk_t *ret;

	if((((uintptr_t) c + sizeof(mem_chunk_t)) & ~((uintptr_t) 4095))
		> ((uintptr_t) c & ~((uintptr_t) 4095)))
	{
		void *ret;
		if(!(ret = physical_alloc())) return NULL;
		bzero(ret, PAGE_SIZE);
	}
	else
		ret = c + sizeof(mem_chunk_t);
	
	c->next = ret;
	ret->prev = c;

	return ret;
}

void *kmalloc(const size_t size)
{
	if(size == 0) return NULL;

	errno = 0;

	static mem_chunk_t *chunks = NULL;
	if(!chunks && !(chunks = new_chunk(chunks)))
	{
		errno = ENOMEM;
		return NULL;
	}

	// TODO
	return NULL;
}
