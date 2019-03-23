#include "memory.h"
#include "../util/util.h"

static uint32_t page_directory[TABLES_COUNT] __attribute__((aligned(4096)));
 __attribute__((aligned(4096)))
static uint32_t kernel_table[PAGES_PER_TABLE];

extern void paging_enable(const void *directory);

void paging_init()
{
	for(size_t i = 0; i < PAGES_PER_TABLE; ++i)
	{
		kernel_table[i] = (i * PAGE_SIZE)
			| (PAGING_TABLE_WRITE | PAGING_TABLE_PRESENT);
	}

	page_directory[0] = (uintptr_t) kernel_table
		| (PAGING_TABLE_WRITE | PAGING_TABLE_PRESENT);

	paging_enable(page_directory);
}

static void set_tables(const size_t table, const size_t count,
	const uint16_t flags)
{
	for(size_t i = table; i < table + count; ++i)
	{
		page_directory[i] = (i * (PAGE_SIZE * PAGES_PER_TABLE)) | flags;
	}
}

static void set_pages(const size_t page, const size_t count,
	const uint16_t tables_flags, const uint16_t pages_flags)
{
	set_tables(page / PAGES_PER_TABLE, count / PAGES_PER_TABLE + 1,
		tables_flags);

	for(size_t i = page; i < page + count; ++i)
	{
		const size_t table = page / PAGES_PER_TABLE;
		const size_t page = page % PAGES_PER_TABLE;
		*((uint32_t *) (((uint32_t *) (page_directory[table]
			& PAGING_ADDR_MASK))[page]
				& PAGING_ADDR_MASK)) = (i * PAGE_SIZE) | pages_flags;
	}
}

void *paging_alloc(const void *hint, const size_t count, const uint16_t flags)
{
	if(count == 0 || !is_aligned(hint, 4096)) return NULL;

	if(hint)
	{
		// TODO
	}

	// TODO
	(void) flags;
	(void) set_pages;

	return NULL;
}

void paging_free(const void *page, const size_t count)
{
	if(!page || count == 0) return;

	// TODO
}
