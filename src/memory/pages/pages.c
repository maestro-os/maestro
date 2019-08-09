#include <memory/pages/pages.h>

// TODO Spinlock

static pages_alloc_t *allocs = NULL;

__attribute__((section("bss")))
static pages_alloc_t *free_list[FREE_LIST_SIZE];

// TODO Split page alloc if needed (on free)

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

	if(!(alloc = kmalloc_zero(sizeof(pages_alloc_t))))
		return NULL;
	order = buddy_get_order(pages * PAGE_SIZE);
	if(!(alloc->buddy = buddy_alloc(order)))
	{
		kfree((void *) alloc);
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
	// TODO Update free list
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
	// TODO Update free list
	return ptr;
}

void *pages_alloc(const size_t n)
{
	int _kmalloc_buddy;
	pages_alloc_t *alloc;
	void *ptr;

	if(n == 0)
		return NULL;
	_kmalloc_buddy = kmalloc_buddy;
	kmalloc_buddy = 1;
	if(!(alloc = find_alloc(n)) && !(alloc = alloc_buddy(n)))
		return NULL;
	ptr = alloc_pages(alloc, n);
	kmalloc_buddy = _kmalloc_buddy;
	return ptr;
}

void *pages_alloc_zero(const size_t n)
{
	void *ptr;

	if((ptr = pages_alloc(n)))
		bzero(ptr, n * PAGE_SIZE);
	return ptr;
}

void pages_free(void *ptr, size_t n)
{
	int _kmalloc_buddy;

	if(!ptr || n == 0)
		return;
	_kmalloc_buddy = kmalloc_buddy;
	kmalloc_buddy = 1;
	// TODO
	(void) ptr;
	(void) n;
	kmalloc_buddy = _kmalloc_buddy;
}
