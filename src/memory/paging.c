#include "memory.h"

static uint32_t page_directory[1024] __attribute__((aligned(4096)));
static uint32_t first_page_table[1024] __attribute__((aligned(4096)));
// TODO Use second table to store pages

static void fill_first_table()
{
	for(size_t i = 0; i < KERNEL_RESERVED_PAGES; ++i)
	{
		first_page_table[i] = (i * 0x1000)
			| (PAGING_TABLE_WRITE | PAGING_TABLE_PRESENT);
	}
}

extern void paging_enable(const void *directory);

void paging_init()
{
	bzero(page_directory, sizeof(page_directory));
	fill_first_table();
	page_directory[0] = ((uintptr_t) first_page_table)
		| PAGING_TABLE_WRITE | PAGING_TABLE_PRESENT;

	paging_enable(page_directory);
}

uint32_t *paging_get_page(const size_t i)
{
	const size_t table = i / 1024;
	const size_t page = i % 1024;
	const uint32_t table_desc = page_directory[table];

	if(!(table_desc & PAGING_TABLE_PRESENT)) return NULL;

	return ((uint32_t *) (table_desc & PAGING_ADDR_MASK)) + page;
}

size_t paging_alloc(const size_t hint, const size_t count,
	const uint16_t flags)
{
	if(count == 0) return 0;

	if(hint)
	{
		// TODO
	}

	// TODO
	(void) flags;

	return 0;
}
