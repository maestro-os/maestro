#include <memory/pages/pages.h>

pages_alloc_t *get_nearest_buddy(void *ptr)
{
	// TODO Avoid doing in linear time
	pages_alloc_t *a;

	if(!ptr)
		return NULL;
	a = allocs;
	while(a)
	{
		if(ptr >= a->buddy && (!a->next_buddy || ptr < a->next_buddy->buddy))
			return a;
		a = a->next_buddy;
	}
	return NULL;
}

void set_next_buddy(pages_alloc_t *alloc, pages_alloc_t *next)
{
	if(!alloc || alloc == next)
		return;
	alloc->next_buddy = next;
}

void delete_buddy(pages_alloc_t *alloc)
{
	pages_alloc_t *next;

	if(!alloc || alloc->buddy_next || alloc->buddy_prev)
		return;
	// TODO Delete every node of the buddy?
	next = alloc->next_buddy;
	set_next_buddy(prev, alloc->next_buddy);
	buddy_free((void *) alloc->buddy);
	kfree((void *) alloc, KMALLOC_BUDDY);
}
