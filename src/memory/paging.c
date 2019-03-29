#include "memory.h"
#include "memory_internal.h"

static inline uint32_t *get_entry(const uint32_t *table, const size_t entry)
{
	return (uint32_t *) (table[entry] & PAGING_ADDR_MASK);
}

bool paging_is_allocated(const uint32_t *directory,
	const void *ptr, const size_t length)
{
	const size_t page = ptr_to_page(ptr);

	for(size_t i = 0; i < length; ++i)
		if(!(*paging_get_page(directory, page + i) & PAGING_PAGE_PRESENT))
			return false;

	return true;
}

static void *paging_find_free(uint32_t *directory, const size_t length)
{
	// TODO
	(void) directory;
	(void) length;

	return NULL;
}

void *paging_alloc(uint32_t *directory, void *hint,
	const size_t length, const paging_flags_t flags)
{
	// TODO
	(void) directory;
	(void) hint;
	(void) length;
	(void) flags;
	(void) paging_find_free;

	return NULL;
}

void paging_free(uint32_t *directory, void *ptr, const size_t length)
{
	// TODO
	(void) directory;
	(void) ptr;
	(void) length;
}

uint32_t *paging_get_page(const uint32_t *directory, const size_t page)
{
	if(!directory) return NULL;

	const size_t t = page / PAGING_DIRECTORY_SIZE;
	const size_t p = page % PAGING_TABLE_SIZE;

	if(!(directory[t] & PAGING_TABLE_PRESENT)) return NULL;
	return get_entry(directory, t) + p;
}

void paging_set_page(uint32_t *directory, const size_t page,
	void *physaddr, const paging_flags_t flags)
{
	if(!directory) return;

	const size_t t = page / PAGING_DIRECTORY_SIZE;
	const size_t p = page % PAGING_TABLE_SIZE;

	if(!(directory[t] & PAGING_TABLE_PRESENT))
	{
		void *ptr;
		if(!(ptr = physical_alloc())) return;
		bzero(ptr, PAGE_SIZE);

		directory[t] = (uintptr_t) ptr
			| PAGING_TABLE_PRESENT | (flags & 0b111111);
	}

	get_entry(directory, t)[p] |= ((uintptr_t) physaddr & PAGING_ADDR_MASK)
		| (flags & PAGING_FLAGS_MASK);
}
