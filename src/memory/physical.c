#include "memory.h"
#include "memory_internal.h"

static size_t total_pages, available_pages, pages_bitmap_size;
static int8_t *pages_bitmap = HEAP_BEGIN;

static size_t upper_division(const size_t n1, const size_t n2)
{
	return (n1 % n2 == 0 ? n1 / n2 : (n1 / n2) + 1);
}

void physical_init()
{
	total_pages = (uintptr_t) memory_end / PAGE_SIZE;
	pages_bitmap_size = upper_division(total_pages, 8);
	available_pages = (memory_end - HEAP_BEGIN) / PAGE_SIZE
		- upper_division(pages_bitmap_size, PAGE_SIZE);

	bzero(pages_bitmap, pages_bitmap_size);
	memset(pages_bitmap, ~0, upper_division(pages_bitmap_size, PAGE_SIZE));
}

size_t physical_available()
{
	return available_pages;
}

static void alloc_hook(void *handle, void *ptr)
{
	*((void **) handle) = ptr;
}

void *physical_alloc()
{
	void *ptr;
	physical_alloc2(1, alloc_hook, &ptr);

	return ptr;
}

// TODO Make atomic
bool physical_alloc2(size_t pages, void (*f)(void *, void *), void *handle)
{
	if(available_pages < pages) return false;

	size_t i = 0, j;

	while(pages)
	{
		while(i < pages_bitmap_size && ~(pages_bitmap[i]) == 0) ++i;

		uint8_t mask = 1;
		j = 0;

		while(mask && (pages_bitmap[i] ^ mask) == 0)
		{
			mask <<= 1;
			++j;
		}

		pages_bitmap[i] |= mask;
		f(handle, (void *) ((uintptr_t) PAGE_SIZE * ((i * 8) + j)));
		--pages;
		--available_pages;
	}

	return true;
}

void physical_free(void *ptr)
{
	const size_t page = (uintptr_t) ptr / PAGE_SIZE;
	const size_t i = page / 8;
	const size_t j = page % 8;

	uint8_t mask = 1;
	for(size_t k = 0; k < j; ++k) mask <<= 1;

	pages_bitmap[i] |= ~mask;
	++available_pages;
}
