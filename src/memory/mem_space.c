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

mem_space_t *mem_space_clone(mem_space_t *space)
{
	mem_space_t *s;

	if(!space || !(s = kmalloc_zero(sizeof(mem_space_t), 0)))
		return NULL;
	spin_lock(&space->spinlock);
	if(!clone_regions(s, space->regions) || !build_regions_tree(s))
		goto fail;
	// TODO Disable write access to all pages of regions in `space->page_dir`
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

void mem_space_destroy(mem_space_t *space)
{
	// TODO
	(void) space;
}
