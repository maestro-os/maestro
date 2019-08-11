#include <memory/pages/pages.h>

static pages_alloc_t *allocs = NULL;

__attribute__((section("bss")))
static pages_alloc_t *free_list[FREE_LIST_SIZE];

static spinlock_t spinlock = 0;

static size_t get_larger_free_order(void)
{
	size_t i = 0, larger = 0;

	while(i < FREE_LIST_SIZE)
	{
		if(free_list[i])
			larger = i;
		++i;
	}
	return larger;
}

static void update_free_list(pages_alloc_t *alloc)
{
	size_t order;

	order = buddy_get_order(alloc->available_pages);
	if(order >= FREE_LIST_SIZE)
		return;
	if(alloc->prev && buddy_get_order(alloc->prev->available_pages) == order)
		return;
	free_list[order] = alloc;
}

static pages_alloc_t *get_nearest_buddy(void *ptr)
{
	// TODO Avoid doing in linear time
	pages_alloc_t *a;

	a = allocs;
	while(a)
	{
		if(ptr >= a->buddy && (!a->next_buddy || ptr < a->next_buddy->buddy))
			return a;
		a = a->next_buddy;
	}
	return NULL;
}

static void set_next_buddy(pages_alloc_t *alloc, pages_alloc_t *next)
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

static void set_prev_buddy(pages_alloc_t *alloc, pages_alloc_t *prev)
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

static pages_alloc_t *find_alloc(const size_t pages)
{
	size_t i;
	pages_alloc_t *a;

	if((i = buddy_get_order(pages * PAGE_SIZE)) >= FREE_LIST_SIZE)
		i = get_larger_free_order();
	if(!(a = free_list[i]))
		a = allocs;
	while(a && a->available_pages < pages)
		a = a->next;
	return a;
}

static void sort_alloc(pages_alloc_t *alloc)
{
	pages_alloc_t *a;

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

static void sort_buddy(pages_alloc_t *alloc)
{
	pages_alloc_t *a;

	if(!(a = get_nearest_buddy(alloc->buddy)) || a == alloc)
		return;
	if(a->buddy < alloc->buddy)
	{
		set_next_buddy(alloc, a->next_buddy);
		set_prev_buddy(alloc, a);
	}
	else
	{
		set_prev_buddy(alloc, a->prev_buddy);
		set_next_buddy(alloc, a);
	}
}

static void delete_buddy(pages_alloc_t *alloc)
{
	if(!alloc->buddy_next && !alloc->buddy_prev)
		return;
	set_next_buddy(alloc->prev_buddy, alloc->next_buddy);
	set_prev_buddy(alloc->next_buddy, alloc->prev_buddy);
	buddy_free((void *) alloc->buddy);
	kfree((void *) alloc, KMALLOC_BUDDY);
}

static void delete_alloc(pages_alloc_t *alloc)
{
	if(alloc->prev)
		alloc->prev->next = alloc->next;
	if(alloc->next)
		alloc->next->prev = alloc->prev;
	if(!alloc->buddy_prev && !alloc->buddy_next)
	{
		delete_buddy(alloc);
		return;
	}
	if(alloc->prev_buddy)
		alloc->prev_buddy->next_buddy = alloc->prev_buddy;
	if(alloc->next_buddy)
		alloc->next_buddy->prev_buddy = alloc->next_buddy;
	if(alloc->buddy_prev)
		alloc->buddy_prev->buddy_next = alloc->buddy_next;
	if(alloc->buddy_next)
		alloc->buddy_next->buddy_prev = alloc->buddy_prev;
	kfree((void *) alloc, KMALLOC_BUDDY);
}

static pages_alloc_t *alloc_buddy(const size_t pages)
{
	pages_alloc_t *alloc;

	if(!(alloc = kmalloc_zero(sizeof(pages_alloc_t), KMALLOC_BUDDY)))
		return NULL;
	if(!(alloc->buddy = buddy_alloc(buddy_get_order(pages * PAGE_SIZE))))
	{
		kfree((void *) alloc, KMALLOC_BUDDY);
		return NULL;
	}
	alloc->ptr = alloc->buddy;
	alloc->buddy_pages = pages;
	alloc->available_pages = pages;
	sort_alloc(alloc);
	update_free_list(alloc);
	sort_buddy(alloc);
	return alloc;
}

static void *alloc_pages(pages_alloc_t *alloc, const size_t pages)
{
	void *ptr;

	ptr = alloc->ptr;
	alloc->ptr += pages * PAGE_SIZE;
	alloc->available_pages -= pages;
	sort_alloc(alloc);
	update_free_list(alloc);
	return ptr;
}

static void split_prev(pages_alloc_t *alloc, void *ptr, const size_t pages)
{
	pages_alloc_t *a;

	if(!(a = kmalloc(sizeof(pages_alloc_t), KMALLOC_BUDDY)))
		return;
	a->next_buddy = alloc->next_buddy;
	a->prev_buddy = alloc->prev_buddy;
	a->buddy_next = alloc;
	if((a->buddy_prev = alloc->buddy_prev))
		alloc->buddy_prev->buddy_next = a;
	alloc->buddy_prev = a;
	a->ptr = ptr;
	a->available_pages = pages;
	sort_alloc(a);
}

static void split_next(pages_alloc_t *alloc, void *ptr, const size_t pages)
{
	pages_alloc_t *a;

	if(!(a = kmalloc(sizeof(pages_alloc_t), KMALLOC_BUDDY)))
		return;
	a->next_buddy = alloc->next_buddy;
	a->prev_buddy = alloc->prev_buddy;
	a->buddy_prev = alloc;
	if((a->buddy_next = alloc->buddy_next))
		alloc->buddy_next->buddy_prev = a;
	alloc->buddy_next = a;
	a->ptr = ptr;
	a->available_pages = pages;
	sort_alloc(a);
}

// TODO Debug every case
static void free_pages(pages_alloc_t *alloc, void *ptr, const size_t pages)
{
	size_t l;

	if(ptr + (PAGE_SIZE * pages) == alloc->ptr)
	{
		alloc->ptr -= (pages * PAGE_SIZE);
		alloc->available_pages += pages;
	}
	else if(ptr + (PAGE_SIZE * pages) < alloc->ptr)
	{
		split_prev(alloc, ptr, pages);
		sort_alloc(alloc);
	}
	else if(ptr > alloc->ptr + (PAGE_SIZE * alloc->available_pages))
	{
		split_next(alloc, ptr, pages);
		sort_alloc(alloc);
	}
	else if(alloc->buddy_next && ptr > alloc->ptr
		&& ptr + (PAGE_SIZE * pages) >= alloc->buddy_next->ptr)
	{
		l = alloc->buddy_next->ptr - (alloc->ptr
			- (alloc->available_pages * PAGE_SIZE));
		alloc->ptr -= l;
		alloc->available_pages += l;
		delete_alloc(alloc->buddy_next);
	}
	if(alloc->available_pages == alloc->buddy_pages
		&& alloc->ptr == alloc->buddy)
		delete_buddy(alloc->buddy);
}

void *pages_alloc(const size_t n)
{
	pages_alloc_t *alloc;
	void *ptr = NULL;

	if(n == 0)
		return NULL;
	if((alloc = find_alloc(n)) || (alloc = alloc_buddy(n)))
		ptr = alloc_pages(alloc, n);
	return ptr;
}

void *pages_alloc_zero(const size_t n)
{
	void *ptr;

	lock(&spinlock);
	if((ptr = pages_alloc(n)))
		bzero(ptr, n * PAGE_SIZE);
	unlock(&spinlock);
	return ptr;
}

void pages_free(void *ptr, const size_t n)
{
	pages_alloc_t *alloc;

	if(!ptr || n == 0)
		return;
	lock(&spinlock);
	if((alloc = get_nearest_buddy(ptr)))
		free_pages(alloc, ptr, n);
	unlock(&spinlock);
}
