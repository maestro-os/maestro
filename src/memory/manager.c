#include "memory.h"

static uint32_t default_directory[1024] __attribute__((aligned(4096)));
static uint32_t *pages_bitmap = HEAP_BEGIN;
static size_t bitmap_len;
static uint32_t *heap_begin;

void mm_init()
{
	paging_create_directory(default_directory);
	// TODO Identity paging on first table

	// TODO paging_enable(default_directory);

	bitmap_len = ((uintptr_t) memory_end / PAGE_SIZE) / (sizeof(uint32_t) * 8);
	heap_begin = pages_bitmap + bitmap_len;
	bzero(pages_bitmap, bitmap_len * sizeof(uint32_t));
	memset(pages_bitmap, ~0, (uintptr_t) HEAP_BEGIN);
}

void *mm_alloc_pages(void *hint, const size_t pages)
{
	if(pages == 0) return NULL;

	// TODO
	(void) hint;

	return NULL;
}

void mm_free_pages(void *ptr, const size_t pages)
{
	if(!ptr || pages == 0) return;

	// TODO
}
