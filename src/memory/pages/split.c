#include <memory/pages/pages.h>

void split_prev(pages_alloc_t *alloc, void *ptr, const size_t pages)
{
	pages_alloc_t *a;

	if(!alloc || !ptr || pages == 0)
		return;
	if(!(a = kmalloc(sizeof(pages_alloc_t), KMALLOC_BUDDY)))
		return;
	a->next_buddy = alloc->next_buddy;
	a->buddy_next = alloc;
	if((a->buddy_prev = alloc->buddy_prev))
		alloc->buddy_prev->buddy_next = a;
	alloc->buddy_prev = a;
	a->ptr = ptr;
	a->available_pages = pages;
	sort_alloc(a);
}

void split_next(pages_alloc_t *alloc, void *ptr, const size_t pages)
{
	pages_alloc_t *a;

	if(!alloc || !ptr || pages == 0)
		return;
	if(!(a = kmalloc(sizeof(pages_alloc_t), KMALLOC_BUDDY)))
		return;
	a->next_buddy = alloc->next_buddy;
	a->buddy_prev = alloc;
	if((a->buddy_next = alloc->buddy_next))
		alloc->buddy_next->buddy_prev = a;
	alloc->buddy_next = a;
	a->ptr = ptr;
	a->available_pages = pages;
	sort_alloc(a);
}
