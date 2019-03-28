#include "memory.h"
#include "memory_internal.h"
#include "../util/util.h"

uint32_t *paging_directory_get_table(uint32_t *directory, const size_t table)
{
	return directory + table;
}

void paging_directory_set_table(uint32_t *directory, const size_t table,
	void *table_ptr, const uint16_t flags)
{
	uint32_t *t = paging_directory_get_table(directory, table);
	*t = ((uintptr_t) table_ptr & PAGING_ADDR_MASK)
		| (flags & PAGING_FLAGS_MASK);
}

uint32_t *paging_table_get_page(uint32_t *table, const size_t page)
{
	return table + page;
}

void paging_table_set_page(uint32_t *table, const size_t page,
	void *physaddr, const uint16_t flags)
{
	uint32_t *p = paging_table_get_page(table, page);
	*p = ((uintptr_t) physaddr & PAGING_ADDR_MASK)
		| (flags & PAGING_FLAGS_MASK);
}

void *paging_physaddr(void *directory, void *virtaddr)
{
	if(!directory) return NULL;

	const size_t dir_index = (uintptr_t) virtaddr >> 22;
	const size_t tab_index = (uintptr_t) virtaddr >> 12 & 0x3ff;

	const uint32_t *t = paging_directory_get_table(directory, dir_index);
	if(!(*t & PAGING_TABLE_PRESENT)) return NULL;

	uint32_t *p = paging_table_get_page((void *) (*t & PAGING_ADDR_MASK),
		tab_index);
	if(!(*p & PAGING_PAGE_PRESENT)) return NULL;

	return (void *) (*p & PAGING_ADDR_MASK);
}

void paging_map(void *directory, void *physaddr, void *virtaddr,
	const uint16_t table_flags, const uint16_t page_flags)
{
	if(!directory || (uintptr_t) physaddr & 12
		|| (uintptr_t )virtaddr & 12) return;

	const size_t dir_index = (uintptr_t) virtaddr >> 22;
	const size_t tab_index = (uintptr_t) virtaddr >> 12 & 0x3ff;

	const uint32_t *t = paging_directory_get_table(directory, dir_index);
	// TODO Check if table is present, else create one
	// TODO Mark table as present
	(void) table_flags;

	uint32_t *p = paging_table_get_page((void *) (*t & PAGING_ADDR_MASK),
		tab_index);

	*p = ((uintptr_t) physaddr | (page_flags & PAGING_FLAGS_MASK));
}
