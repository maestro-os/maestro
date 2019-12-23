#include <memory/memory.h>

mem_space_t *mem_space_init(void)
{
	mem_space_t *s;

	// TODO Slab allocation
	if(!(s = kmalloc_zero(sizeof(mem_space_t), 0)))
		return NULL;
	if(!(s->page_dir = vmem_init()))
	{
		kfree(s, 0);
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
	size_t i;
	void *ptr;

	while(r)
	{
		ptr = r->start;
		while(i < r->pages)
		{
			*vmem_resolve(page_dir, ptr + (i * PAGE_SIZE))
				&= ~PAGING_PAGE_WRITE;
			++i;
		}
		r = r->next;
	}
}

mem_space_t *mem_space_clone(mem_space_t *space)
{
	mem_space_t *s;

	// TODO Slab allocation
	if(!space || !(s = kmalloc_zero(sizeof(mem_space_t), 0)))
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
	kfree(s, 0);
	// TODO Free all, remove links, etc...
	spin_unlock(&space->spinlock);
	return NULL;
}

void *mem_space_alloc(mem_space_t *space, size_t pages)
{
	// TODO
	(void) space;
	(void) pages;
	return NULL;
}

void *mem_space_alloc_stack(mem_space_t *space, size_t max_pages)
{
	// TODO
	(void) space;
	(void) max_pages;
	return NULL;
}

void mem_space_free(mem_space_t *space, void *ptr, size_t pages)
{
	// TODO
	(void) space;
	(void) ptr;
	(void) pages;
}

void mem_space_free_stack(mem_space_t *space, void *stack)
{
	// TODO
	(void) space;
	(void) stack;
}

int mem_space_can_access(mem_space_t *space, const void *ptr, size_t size)
{
	// TODO
	(void) space;
	(void) ptr;
	(void) size;
	return 0;
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
	// TODO rb_tree_freeall(space->tree);
	kfree(space, 0);
}
