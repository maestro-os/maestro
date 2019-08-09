#ifndef PAGES_H
# define PAGES_H

# include <memory/memory.h>

# define FREE_LIST_SIZE	16

typedef struct pages_alloc
{
	struct pages_alloc *next;
	struct pages_alloc *prev;
	struct pages_alloc *next_buddy;
	struct pages_alloc *prev_buddy;

	void *buddy;
	size_t buddy_pages;
	
	void *ptr;
	size_t available_pages;
} pages_alloc_t;

void *pages_alloc(size_t n);
void *pages_alloc_zero(size_t n);
void pages_free(void *ptr, size_t n);

#endif
