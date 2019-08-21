#include <memory/pages/pages.h>

void sort_alloc(pages_alloc_t *alloc)
{
	pages_alloc_t *a;

	if(!alloc)
		return;
	if(alloc->next)
		alloc->next->prev = alloc->prev;
	if(alloc->prev)
		alloc->prev->next = alloc->next;
	if((a = find_alloc(alloc->available_pages)))
	{
		if(a != alloc)
		{
			alloc->next = a;
			alloc->prev = a->prev;
			if(a->next)
				a->next->prev = alloc;
			if(a->prev)
				a->prev->next = alloc;
		}
	}
	else
	{
		alloc->next = NULL;
		alloc->prev = NULL;
		allocs = alloc;
	}
	update_free_list(alloc);
}

void sort_buddy(pages_alloc_t *alloc)
{
	pages_alloc_t *a;

	if(!alloc || !(a = get_nearest_buddy(alloc->buddy)) || a == alloc)
		return;
	if(a->buddy < alloc->buddy)
		set_next_buddy(alloc, a->next_buddy);
	else
		set_next_buddy(alloc, a);
}
