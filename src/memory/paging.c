#include "memory.h"

static uint32_t page_directory[1024] __attribute__((aligned(4096)));
static uint32_t first_page_table[1024] __attribute__((aligned(4096)));

static inline void blank_directory()
{
	memset(page_directory, PAGING_DIR_WRITE, sizeof(page_directory));
}

static void fill_first_table()
{
	for(size_t i = 0; i < 1024; ++i)
	{
		first_page_table[i] = (i * 0x1000)
			| (PAGING_TABLE_WRITE | PAGING_TABLE_PRESENT);
	}
}

extern void paging_enable(const void *directory);

void paging_init()
{
	blank_directory();

	fill_first_table();
	paging_set_table(0, first_page_table,
		PAGING_DIR_WRITE | PAGING_DIR_PRESENT);

	paging_enable(page_directory);
}

void *paging_get_addr(const page_t *page)
{
	if(!page) return NULL;
	return (void *) (((page->directory_entry * 1024) + page->table_entry) * 4);
}

page_t paging_get_page(const void *addr)
{
	page_t page;
	page.directory_entry = ((uintptr_t) addr / 4) / 1024;
	page.table_entry = ((uintptr_t) addr / 4) % 1024;

	return page;
}

uint32_t *paging_get_table(const size_t i)
{
	return page_directory + i;
}

void paging_set_table(const size_t i, const uint32_t *table,
	const uint16_t flags)
{
	page_directory[i] = ((uint32_t) table) | flags;
}
