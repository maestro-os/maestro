#ifndef PAGES_H
# define PAGES_H

# include <memory/memory.h>

# define FREE_LIST_SIZE	16

// TODO Rename buddy to block

typedef struct pages_alloc
{
	struct pages_alloc *next;
	struct pages_alloc *prev;
	struct pages_alloc *buddy_next;
	struct pages_alloc *buddy_prev;
	struct pages_alloc *next_buddy;
	struct pages_alloc *prev_buddy;

	void *buddy;
	size_t buddy_pages;
	
	void *ptr;
	size_t available_pages;
} pages_alloc_t;

extern pages_alloc_t *allocs;
extern pages_alloc_t *free_list[FREE_LIST_SIZE];

pages_alloc_t *get_nearest_buddy(void *ptr);
void set_next_buddy(pages_alloc_t *alloc, pages_alloc_t *next);
void set_prev_buddy(pages_alloc_t *alloc, pages_alloc_t *prev);
void delete_buddy(pages_alloc_t *alloc);

void sort_alloc(pages_alloc_t *alloc);
void sort_buddy(pages_alloc_t *alloc);

void split_prev(pages_alloc_t *alloc, void *ptr, size_t pages);
void split_next(pages_alloc_t *alloc, void *ptr, size_t pages);

pages_alloc_t *find_alloc(size_t pages);
void update_free_list(pages_alloc_t *alloc);

void *pages_alloc(size_t n);
void *pages_alloc_zero(size_t n);
void pages_free(void *ptr, size_t n);

#endif
