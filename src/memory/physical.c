#include "memory.h"
#include "memory_internal.h"

static size_t total_pages, available_pages, free_base, pages_bitmap_size;
static int8_t *pages_bitmap = HEAP_BEGIN;

void physical_init()
{
	total_pages = (uintptr_t) memory_end / PAGE_SIZE;
	available_pages = (memory_end - HEAP_BEGIN) / PAGE_SIZE;
	free_base = total_pages - available_pages;
	pages_bitmap_size = (total_pages % 8 == 0
		? (total_pages / 8) : (total_pages / 8) + 1);

	bzero(pages_bitmap, pages_bitmap_size);
	memset(pages_bitmap, ~((uint8_t) 0), pages_bitmap_size / 8);
	if(pages_bitmap_size % 8 != 0)
		pages_bitmap[pages_bitmap_size / 8] = ~((uint8_t) 0);

	available_pages -= (pages_bitmap_size + 1) / 8;
	free_base += pages_bitmap_size / PAGE_SIZE;
	if(free_base == 0)
		++free_base;
}

size_t physical_available()
{
	return available_pages;
}

void *physical_alloc()
{
	if(available_pages == 0) return NULL;

	size_t i = 0;
	size_t j = 0;

	while(i < pages_bitmap_size && ~(pages_bitmap[i]) == 0)
		++i;

	if(i >= pages_bitmap_size) return NULL;

	uint8_t mask = 1;

	while(mask && (pages_bitmap[i] ^ mask) == 0)
	{
		mask <<= 1;
		++j;
	}

	if(!mask) return NULL;

	return (void *) ((uintptr_t) PAGE_SIZE * ((i * 8) + j));
}

void physical_free(void *ptr)
{
	const size_t page = (uintptr_t) ptr / PAGE_SIZE;
	const size_t i = page / 8;
	const size_t j = page % 8;

	uint8_t mask = 1;
	for(size_t k = 0; k < j; ++k)
		mask <<= 1;

	pages_bitmap[i] |= mask;
}
