#include "memory.h"
#include "../util/util.h"

static uint32_t page_directory[TABLES_COUNT] __attribute__((aligned(4096)));
static uint32_t *tables = TABLES_ADDR;

extern void paging_enable(const void *directory);

void paging_init()
{
	bzero(page_directory, sizeof(page_directory));
	bzero(tables, TABLES_SIZE);

	for(size_t i = 0; i < TABLES_COUNT; ++i)
		page_directory[i] = (((uintptr_t) TABLES_ADDR + (PAGE_SIZE * i))
			& PAGING_ADDR_MASK) | (PAGING_TABLE_PRESENT | PAGING_TABLE_WRITE);

	size_t i = 0;

	while(i < (PAGES_PER_TABLE * 2))
	{
		tables[i] = (i * PAGE_SIZE)
			| (PAGING_PAGE_WRITE | PAGING_PAGE_PRESENT);
		++i;
	}

	while(i < (TABLES_COUNT * PAGES_PER_TABLE))
	{
		tables[i] = (i * PAGE_SIZE);
		++i;
	}

	paging_enable(page_directory);
}

static bool pages_fit(const size_t page, const size_t count)
{
	for(size_t i = page; i < page + count; ++i)
		if(tables[i] & PAGING_PAGE_PRESENT) return false;

	return true;
}

static size_t find_free_pages(const size_t hint, const size_t count)
{
	if(pages_fit(hint, count)) return hint;

	size_t i = 0;

	while(i < TOTAL_PAGES)
	{
		if(pages_fit(i, count)) return i;

		while(i < TOTAL_PAGES && !(tables[i] & PAGING_PAGE_PRESENT)) ++i;
		while(i < TOTAL_PAGES && tables[i] & PAGING_PAGE_PRESENT) ++i;
	}

	return (size_t) -1;
}

static void set_pages(const size_t page, const size_t count,
	const uint16_t tables_flags, const uint16_t pages_flags)
{
	const size_t begin_table = page / PAGES_PER_TABLE;
	const size_t end_table = ((page + count) / PAGES_PER_TABLE) + 1;

	for(size_t i = begin_table; i < TOTAL_PAGES && i < end_table; ++i)
		page_directory[i] = (page_directory[i] & PAGING_ADDR_MASK)
			| (tables_flags & PAGING_FLAGS_MASK);

	for(size_t i = page; i < TOTAL_PAGES && i < page + count; ++i)
		tables[i] = (tables[i] & PAGING_ADDR_MASK)
			| (pages_flags & PAGING_FLAGS_MASK);
}

static uint16_t get_tables_flags(const uint16_t page_flags)
{
	uint16_t flags = 0;

	if(page_flags & PAGING_PAGE_CACHE_DISABLE)
		flags |= PAGING_TABLE_CACHE_DISABLE;

	if(page_flags & PAGING_PAGE_WRITE_THROUGH)
		flags |= PAGING_TABLE_WRITE_THROUGH;

	if(page_flags & PAGING_PAGE_USER)
		flags |= PAGING_TABLE_USER;

	if(page_flags & PAGING_PAGE_WRITE)
		flags |= PAGING_TABLE_WRITE;

	if(page_flags & PAGING_PAGE_PRESENT)
		flags |= PAGING_TABLE_PRESENT;

	return flags;
}

void *paging_alloc(const void *hint, const size_t count, const uint16_t flags)
{
	if(count == 0 || !is_aligned(hint, 4096)) return NULL;

	const size_t begin_page = find_free_pages((size_t) hint / PAGE_SIZE, count);
	if(begin_page == (size_t) -1) return NULL;

	const uint16_t f = flags | PAGING_PAGE_PRESENT;
	const uint16_t table_flags = get_tables_flags(f);
	set_pages(begin_page, count, table_flags, f);

	return (void *) (begin_page * PAGE_SIZE);
}

void paging_free(const void *page, const size_t count)
{
	if(!page || count == 0) return;
	set_pages((size_t) page / PAGE_SIZE, count, 0, 0);
}
