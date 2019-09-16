#include <memory/pages/pages.h>

pages_alloc_t *allocs = NULL;

__ATTR_BSS
pages_alloc_t *free_list[FREE_LIST_SIZE];

static spinlock_t spinlock = 0;

static size_t get_largest_free_order(void)
{
	size_t i = 0, largest = 0;

	while(i < FREE_LIST_SIZE)
	{
		if(free_list[i])
			largest = i;
		++i;
	}
	return largest;
}

void update_free_list(pages_alloc_t *alloc)
{
	size_t order;

	order = buddy_get_order(alloc->available_pages);
	if(order >= FREE_LIST_SIZE)
		return;
	if(alloc->prev && buddy_get_order(alloc->prev->available_pages) == order)
		return;
	free_list[order] = alloc;
}

pages_alloc_t *find_alloc(const size_t pages)
{
	size_t i;
	pages_alloc_t *a;

	if((i = buddy_get_order(pages * PAGE_SIZE)) >= FREE_LIST_SIZE)
		i = get_largest_free_order();
	if(!(a = free_list[i]))
		a = allocs;
	while(a && a->available_pages < pages)
		a = a->next;
	return a;
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

	spin_lock(&spinlock);
	if((ptr = pages_alloc(n)))
		bzero(ptr, n * PAGE_SIZE);
	spin_unlock(&spinlock);
	return ptr;
}

void pages_free(void *ptr, const size_t n)
{
	pages_alloc_t *alloc;

	if(!ptr || n == 0)
		return;
	spin_lock(&spinlock);
	if((alloc = get_nearest_buddy(ptr)))
		free_pages(alloc, ptr, n);
	spin_unlock(&spinlock);
}
