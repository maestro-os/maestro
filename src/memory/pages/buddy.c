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
	pages_alloc_t *a;

	if(!alloc || alloc == next)
		return;
	a = next;
	while(a)
	{
		a->prev_buddy = alloc;
		a = a->buddy_next;
	}
	alloc->next_buddy = next;
}

void set_prev_buddy(pages_alloc_t *alloc, pages_alloc_t *prev)
{
	pages_alloc_t *a;

	if(!alloc || alloc == prev)
		return;
	a = prev;
	while(a)
	{
		a->next_buddy = alloc;
		a = a->buddy_next;
	}
	alloc->prev_buddy = prev;
}

void delete_buddy(pages_alloc_t *alloc)
{
	pages_alloc_t *prev, *next;

	if(!alloc || alloc->buddy_next || alloc->buddy_prev)
		return;
	// TODO Delete every node of the buddy?
	prev = alloc->prev_buddy;
	next = alloc->next_buddy;
	set_next_buddy(prev, alloc->next_buddy);
	set_prev_buddy(next, alloc->prev_buddy);
	buddy_free((void *) alloc->buddy);
	kfree((void *) alloc, KMALLOC_BUDDY);
}
