#include <memory/memory.h>
#include <kernel.h>

static cache_t *mem_space_cache;

static void global_init(void)
{
	if(!(mem_space_cache = cache_create("mem_space", sizeof(mem_space_t), 64,
		bzero, NULL)))
		PANIC("Failed to initialize mem_space cache!", 0);
}

mem_space_t *mem_space_init(void)
{
	static int init = 0;
	mem_space_t *s;

	if(!init)
	{
		global_init();
		init = 1;
	}
	if(!(s = cache_alloc(mem_space_cache)))
		return NULL;
	if(!(s->page_dir = vmem_init()))
	{
		cache_free(mem_space_cache, s);
		return NULL;
	}
	return s;
}

static mem_region_t *clone_region(mem_space_t *space, mem_region_t *r)
{
	size_t bitfield_size;
	mem_region_t *new;

	bitfield_size = BITFIELD_SIZE(r->pages);
	if(!(new = kmalloc_zero(sizeof(mem_region_t) + bitfield_size, 0)))
		return NULL;
	new->mem_space = space;
	new->flags = r->flags;
	new->start = r->start;
	new->pages = r->pages;
	new->used_pages = r->used_pages;
	memcpy(new->use_bitfield, r->use_bitfield, bitfield_size);
	if((new->next_shared = r->next_shared))
		r->next_shared->prev_shared = new;
	if((new->prev_shared = r))
		r->next_shared = new;
	return new;
}

static int clone_regions(mem_space_t *dest, mem_region_t *src)
{
	mem_region_t *r;
	mem_region_t *new;
	mem_region_t *last = NULL;

	r = src;
	while(r)
	{
		if(!(new = clone_region(dest, r)))
		{
			// TODO Free all
			return 0;
		}
		if(last)
		{
			last->next = new;
			last = new;
		}
		else
			last = dest->regions = new;
		r = r->next;
	}
	return 1;
}

static int build_regions_tree(mem_space_t *space)
{
	// TODO
	(void) space;
	return 1;
}

static void regions_disable_write(mem_region_t *r, vmem_t page_dir)
{
	void *ptr;
	size_t i;

	for(; r; r = r->next)
	{
		if(!(r->flags & MEM_REGION_FLAG_WRITE))
			continue;
		ptr = r->start;
		for(i = 0; i < r->pages; ++i)
			*vmem_resolve(page_dir, ptr + (i * PAGE_SIZE))
				&= ~PAGING_PAGE_WRITE;
	}
}

mem_space_t *mem_space_clone(mem_space_t *space)
{
	mem_space_t *s;

	if(!space || !(s = cache_alloc(mem_space_cache)))
		return NULL;
	spin_lock(&space->spinlock);
	if(!clone_regions(s, space->regions) || !build_regions_tree(s))
		goto fail;
	regions_disable_write(space->regions, space->page_dir);
	if(!(s->page_dir = vmem_clone(space->page_dir)))
		goto fail;
	spin_unlock(&space->spinlock);
	return s;

fail:
	cache_free(mem_space_cache, s);
	// TODO Free all, remove links, etc...
	spin_unlock(&space->spinlock);
	return NULL;
}

static mem_region_t *region_create(mem_space_t *space,
	const size_t pages, const int stack)
{
	mem_region_t *r;

	if(pages == 0)
		return NULL;
	if(!(r = kmalloc(sizeof(mem_region_t) + BITFIELD_SIZE(pages), 0)))
		return NULL;
	r->mem_space = space;
	if(stack)
		r->flags |= MEM_REGION_FLAG_STACK;
	r->start = NULL; // TODO Find available zone using the tree
	r->pages = pages;
	return r;
}

void *mem_space_alloc(mem_space_t *space, size_t pages)
{
	mem_region_t *r;

	// TODO Return NULL if available physical pages count is too low
	if(!(r = region_create(space, pages, 0)))
		return NULL;
	r->used_pages = r->pages;
	bitfield_set_range(r->use_bitfield, 0, r->pages);
	r->next = space->regions;
	space->regions = r;
	// TODO Insert in tree
	return r->start;
}

void *mem_space_alloc_stack(mem_space_t *space, size_t max_pages)
{
	mem_region_t *r;

	// TODO Return NULL if available physical pages count is too low
	if(!(r = region_create(space, max_pages, 1)))
		return NULL;
	r->next = space->regions;
	space->regions = r;
	// TODO Insert in tree
	return r->start + (r->pages * PAGE_SIZE) - 1;
}

static void region_free(mem_region_t *region)
{
	size_t i;

	if(!region->prev_shared && !region->next_shared)
	{
		i = 0;
		while(i < region->pages)
		{
			if(bitfield_get(region->use_bitfield, i))
				buddy_free(region->start + (i * PAGE_SIZE));
			++i;
		}
	}
	else
	{
		if(region->prev_shared)
			region->prev_shared->next_shared = region->next_shared;
		if(region->next_shared)
			region->next_shared->prev_shared = region->prev_shared;
	}
	kfree(region, 0);
}

void mem_space_free(mem_space_t *space, void *ptr, size_t pages)
{
	if(!space || !ptr || pages == 0)
		return;
	// TODO Find region using tree and free it
}

void mem_space_free_stack(mem_space_t *space, void *stack)
{
	if(!space || !stack)
		return;
	// TODO Find region using tree and free it
}

int mem_space_can_access(mem_space_t *space, const void *ptr, size_t size)
{
	if(!space || !ptr)
		return 0;
	// TODO
	(void) size;
	return 0;
}

int mem_space_handle_page_fault(mem_space_t *space)
{
	if(!space)
		return 0;
	// TODO Check if virtual page is allocated
	// TODO Allocate and map a physical page if needed
	// TODO Return 0 if page isn't accessible or 1 if accessible
	return 0;
}

void mem_space_destroy(mem_space_t *space)
{
	mem_region_t *r, *next;

	if(!space)
		return;
	r = space->regions;
	while(r)
	{
		next = r->next;
		region_free(r);
		r = next;
	}
	rb_tree_freeall(space->tree, NULL);
	cache_free(mem_space_cache, space);
}
