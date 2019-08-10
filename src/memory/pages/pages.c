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

static void update_free_list(const size_t order, pages_alloc_t *alloc)
{
	if(order >= FREE_LIST_SIZE)
		return;
	if(alloc->prev && buddy_get_order(alloc->available_pages) == order)
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
		if(ptr >= a->buddy && (!a->next_buddy && ptr < a->next_buddy->buddy))
			return a;
		a = a->next;
	}
	return NULL;
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

static pages_alloc_t *alloc_buddy(const size_t pages)
{
	pages_alloc_t *alloc;
	size_t order;
	pages_alloc_t *a;

	if(!(alloc = kmalloc_zero(sizeof(pages_alloc_t), KMALLOC_BUDDY)))
		return NULL;
	order = buddy_get_order(pages * PAGE_SIZE);
	if(!(alloc->buddy = buddy_alloc(order)))
	{
		kfree((void *) alloc, KMALLOC_BUDDY);
		return NULL;
	}
	alloc->ptr = alloc->buddy;
	alloc->buddy_pages = pages;
	alloc->available_pages = pages;
	// TODO Use free list for fast insertion?
	if((a = allocs))
	{
		while(a && a->next && a->next->available_pages < pages)
			a = a->next;
		alloc->next = a->next;
		alloc->prev = a;
		alloc->prev->next = alloc;
		alloc->next->prev = alloc;
	}
	else
		allocs = alloc;
	update_free_list(order, alloc);
	if((a = get_nearest_buddy(alloc->buddy)))
	{
		if(a->buddy < alloc->buddy)
		{
			if((alloc->next_buddy = a->next))
				alloc->next_buddy->prev_buddy = alloc;
			if((alloc->prev_buddy = a))
				alloc->prev_buddy->next_buddy = alloc;
		}
		else
		{
			if((alloc->prev_buddy = a->prev))
				alloc->prev_buddy->next_buddy = alloc;
			if((alloc->next_buddy = a))
				alloc->next_buddy->prev_buddy = alloc;
		}
	}
	return alloc;
}

static void *alloc_pages(pages_alloc_t *alloc, const size_t pages)
{
	void *ptr;
	pages_alloc_t *a;

	ptr = alloc->ptr;
	alloc->ptr += pages * PAGE_SIZE;
	alloc->available_pages -= pages;
	// TODO Use free list for fast insertion?
	a = alloc;
	while(a && a->available_pages > alloc->available_pages)
		a = a->prev;
	if(a)
	{
		if(alloc->next)
			alloc->next->prev = alloc->prev;
		if(alloc->prev)
			alloc->prev->next = alloc->next;
		alloc->next = a->next;
		alloc->prev = a;
		alloc->next->prev = alloc;
		alloc->prev->next = alloc;
	}
	else
	{
		alloc->next = allocs;
		allocs = alloc;
	}
	update_free_list(buddy_get_order(alloc->available_pages), alloc);
	return ptr;
}

static void free_pages(pages_alloc_t *alloc, void *ptr, const size_t pages)
{
	if(ptr < alloc->ptr && ptr + (PAGE_SIZE * pages) < alloc->ptr)
	{
		// TODO split to next
	}
	else if(ptr > alloc->ptr + (PAGE_SIZE * alloc->available_pages))
	{
		// TODO split to prev
	}
	else if(alloc->next && ptr > alloc->ptr
		&& ptr + (PAGE_SIZE * pages) >= alloc->next->ptr)
	{
		// TODO merge with next
	}
}

void *pages_alloc(const size_t n)
{
	pages_alloc_t *alloc;
	void *ptr = NULL;

	if(n == 0)
		return NULL;
	if((alloc = find_alloc(n)) && !(alloc = alloc_buddy(n)))
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
