#include "memory.h"

static uint32_t page_directory[1024] __attribute__((aligned(4096)));
static uint32_t first_page_table[1024] __attribute__((aligned(4096)));

static inline void paging_blank_directory()
{
	memset(page_directory, 0b10, sizeof(page_directory));
}

static void paging_enable()
{
	// TODO
}

void paging_init()
{
	paging_blank_directory();
	// TODO
	(void) first_page_table;

	paging_enable();
}

void *paging_get_addr(const size_t directory_entry,
	const size_t page_entry)
{
	return (void *) (((directory_entry * 1024) + page_entry) * 4);
}
