#include <memory/memory.h>
#include <kernel.h>

// TODO Spinlock

static cache_t *mem_space_cache;
static cache_t *mem_gap_cache;

static void global_init(void)
{
	if(!(mem_space_cache = cache_create("mem_space", sizeof(mem_space_t), 64,
		bzero, NULL)))
		PANIC("Failed to initialize mem_space cache!", 0);
	if(!(mem_gap_cache = cache_create("mem_gap", sizeof(mem_gap_t), 64,
		bzero, NULL)))
		PANIC("Failed to initialize mem_gap cache!", 0);
}

static int region_cmp(void *r0, void *r1)
{
	return (uintptr_t) ((mem_region_t *) r1)->begin
		- (uintptr_t) ((mem_region_t *) r0)->begin;
}

static int gap_cmp(void *r0, void *r1)
{
	return (uintptr_t) ((mem_gap_t *) r1)->pages
		- (uintptr_t) ((mem_gap_t *) r0)->pages;
}

/*
 Creates the memory gap for the beginning

 First and last page are not included for NULL pointer
 and to avoid overflow when aligning up
 */
static int init_gaps(mem_space_t *s)
{
	if(!(s->gaps = cache_alloc(mem_gap_cache)))
		return 0;
	s->gaps->begin = (void *) 0x1000;
	s->gaps->pages = 0xffffe;
	// TODO Kernel code/syscall stub should not be inside a gap
	errno = 0;
	avl_tree_insert(&s->free_tree, s->gaps, region_cmp);
	if(errno)
	{
		cache_free(mem_gap_cache, s->gaps);
		return 0;
	}
	return 1;
}

mem_space_t *mem_space_init(void)
{
	static int init = 0;
	mem_space_t *s;

	if(unlikely(!init))
	{
		global_init();
		init = 1;
	}
	if(!(s = cache_alloc(mem_space_cache)))
		return NULL;
	if(!(init_gaps(s)))
	{
		cache_free(mem_space_cache, s);
		return NULL;
	}
	if(!(s->page_dir = vmem_init()))
	{
		cache_free(mem_gap_cache, s->gaps);
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
	new->begin = r->begin;
	new->pages = r->pages;
	new->used_pages = r->used_pages;
	memcpy(new->use_bitfield, r->use_bitfield, bitfield_size);
	if((new->next_shared = r->next_shared))
		r->next_shared->prev_shared = new;
	if((new->prev_shared = r))
		r->next_shared = new;
	return new;
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
				buddy_free(region->begin + (i * PAGE_SIZE));
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

static void remove_regions(mem_region_t *r)
{
	mem_region_t *next;

	while(r)
	{
		next = r->next;
		region_free(r);
		r = next;
	}
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
			remove_regions(dest->regions);
			dest->regions = NULL;
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

static void gap_free(mem_gap_t *gap)
{
	// TODO
	(void) gap;
}

static void remove_gaps(mem_gap_t *g)
{
	mem_gap_t *next;

	while(g)
	{
		next = g->next;
		gap_free(g);
		g = next;
	}
}

static int clone_gaps(mem_space_t *dest, mem_gap_t *src)
{
	mem_gap_t *g;
	mem_gap_t *new;
	mem_gap_t *last = NULL;

	g = src;
	while(g)
	{
		if(!(new = cache_alloc(mem_gap_cache)))
		{
			remove_gaps(dest->gaps);
			dest->gaps = NULL;
			return 0;
		}
		new->prev = last;
		new->begin = g->begin;
		new->pages = g->pages;
		if(last)
		{
			last->next = new;
			last = new;
		}
		else
			last = dest->gaps = new;
		g = g->next;
	}
	return 1;
}

static int build_trees(mem_space_t *space)
{
	mem_region_t *r;
	mem_gap_t *g;

	r = space->regions;
	errno = 0;
	while(r)
	{
		avl_tree_insert(&space->used_tree, r, region_cmp);
		if(errno)
			goto fail;
		r = r->next;
	}
	g = space->gaps;
	while(g)
	{
		avl_tree_insert(&space->free_tree, g, gap_cmp);
		if(errno)
			goto fail;
		g = g->next;
	}
	return 1;

fail:
	avl_tree_freeall(&space->used_tree, NULL);
	avl_tree_freeall(&space->free_tree, NULL);
	return 0;
}

static void regions_disable_write(mem_region_t *r, vmem_t page_dir)
{
	void *ptr;
	size_t i;
	uint32_t *entry;

	for(; r; r = r->next)
	{
		if(!(r->flags & MEM_REGION_FLAG_USER))
			continue;
		if(!(r->flags & MEM_REGION_FLAG_WRITE))
			continue;
		ptr = r->begin;
		for(i = 0; i < r->pages; ++i)
		{
			if((entry = vmem_resolve(page_dir, ptr + (i * PAGE_SIZE))))
				*entry &= ~PAGING_PAGE_WRITE;
		}
	}
}

static int convert_flags(const int reg_flags)
{
	int flags = 0;

	if(reg_flags & MEM_REGION_FLAG_WRITE)
		flags |= PAGING_PAGE_WRITE;
	if(reg_flags & MEM_REGION_FLAG_USER)
		flags |= PAGING_PAGE_USER;
	return flags;
}

static int preallocate_kernel_stack(mem_space_t *space, mem_region_t *r)
{
	void *i, *ptr;

	i = r->begin;
	while(i < r->begin + r->pages * PAGE_SIZE)
	{
		if(!(ptr = buddy_alloc_zero(0)))
		{
			// TODO Free all
			return 0;
		}
		vmem_map(space->page_dir, ptr, i, convert_flags(r->flags));
		i += PAGE_SIZE;
	}
	return 1;
}

mem_space_t *mem_space_clone(mem_space_t *space)
{
	mem_space_t *s;
	mem_region_t *r;

	if(!space || !(s = cache_alloc(mem_space_cache)))
		return NULL;
	spin_lock(&space->spinlock);
	if(!clone_regions(s, space->regions)
		|| !clone_gaps(s, space->gaps) || !build_trees(s))
		goto fail;
	regions_disable_write(space->regions, space->page_dir);
	if(!(s->page_dir = vmem_clone(space->page_dir)))
		goto fail;
	r = s->regions;
	while(r)
	{
		if(!(r->flags & MEM_REGION_FLAG_USER)
			&& r->flags & MEM_REGION_FLAG_STACK)
		{
			if(!preallocate_kernel_stack(s, r))
				goto fail;
		}
		r = r->next;
	}
	spin_unlock(&space->spinlock);
	return s;

fail:
	cache_free(mem_space_cache, s);
	// TODO Free all, remove links, etc...
	spin_unlock(&space->spinlock);
	return NULL;
}

static avl_tree_t *find_gap(avl_tree_t *n, const size_t pages)
{
	if(!n || pages == 0)
		return NULL;
	while(1)
	{
		if(n->left && ((mem_gap_t *) n->left->value)->pages >= pages)
			n = n->left;
		else if(n->right && ((mem_gap_t *) n->right->value)->pages < pages)
			n = n->right;
		else
			break;
	}
	return n;
}

static void shrink_gap(avl_tree_t **tree, avl_tree_t *gap, const size_t pages)
{
	mem_gap_t *g;

	if(!gap || pages == 0)
		return;
	g = gap->value;
	// TODO Error if pages > gap->pages? (shouldn't be possible)
	if(g->pages <= pages)
	{
		if(g->prev)
			g->prev->next = g->next;
		if(g->next)
			g->next->prev = g->prev;
		avl_tree_delete(tree, gap);
		cache_free(mem_gap_cache, g);
		return;
	}
	g->begin += pages * PAGE_SIZE;
	g->pages -= pages;
}

static mem_region_t *region_create(mem_space_t *space,
	const size_t pages, const int flags)
{
	mem_region_t *r;
	avl_tree_t *gap;

	if(pages == 0)
		return NULL;
	if(!(r = kmalloc(sizeof(mem_region_t) + BITFIELD_SIZE(pages), 0)))
		return NULL;
	if(!(gap = find_gap(space->free_tree, pages)))
	{
		kfree(r, 0);
		return NULL;
	}
	r->mem_space = space;
	r->flags = flags;
	r->begin = ((mem_gap_t *) gap->value)->begin;
	r->pages = pages;
	r->used_pages = r->pages;
	bitfield_set_range(r->use_bitfield, 0, r->pages);
	if(!(flags & MEM_REGION_FLAG_USER) && (flags & MEM_REGION_FLAG_STACK))
	{
		if(!preallocate_kernel_stack(space, r))
		{
			kfree(r, 0);
			return NULL;
		}
	}
	errno = 0;
	avl_tree_insert(&space->used_tree, r, region_cmp);
	if(errno)
	{
		// TODO If preallocated kernel_stack, free it
		kfree(r, 0);
		return NULL;
	}
	shrink_gap(&space->free_tree, gap, pages);
	return r;
}

void *mem_space_alloc(mem_space_t *space, const size_t pages, const int flags)
{
	mem_region_t *r;

	// TODO Return NULL if available physical pages count is too low
	if(!(r = region_create(space, pages, flags)))
		return NULL;
	r->next = space->regions;
	space->regions = r;
	if(flags & MEM_REGION_FLAG_STACK)
		return r->begin + (r->pages * PAGE_SIZE) - 1;
	else
		return r->begin;
}

static mem_region_t *find_region(avl_tree_t *n, void *ptr)
{
	mem_region_t *r = NULL;

	if(!ptr)
		return NULL;
	while(n)
	{
		r = n->value;
		if(r->begin >= ptr)
			n = n->left;
		else if(r->begin < ptr)
			n = n->right;
		else
			break;
	}
	if(!r)
		return NULL;
	if(ptr >= r->begin && ptr < r->begin + r->pages * PAGE_SIZE)
		return r;
	return NULL;
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

int mem_space_can_access(mem_space_t *space, const void *ptr, const size_t size,
	const int write)
{
	void *i, *end;
	mem_region_t *r;

	if(!space || !ptr)
		return 0;
	i = ALIGN_DOWN(ptr, PAGE_SIZE);
	end = ALIGN_UP(ptr + size, PAGE_SIZE);
	while(i < end)
	{
		if(!(r = find_region(space->used_tree, i)))
			return 0;
		if(write && !(r->flags & MEM_REGION_FLAG_WRITE))
			return 0;
		while(i < r->begin + r->pages)
		{
			if(!bitfield_get(r->use_bitfield,
				(uintptr_t) (i - r->begin) / PAGE_SIZE))
				return 0;
			i += PAGE_SIZE;
		}
	}
	return 1;
}

static void update_share(mem_region_t *r)
{
	uint32_t *entry;

	if(r->prev_shared || r->next_shared || !(r->flags & MEM_REGION_FLAG_WRITE))
		return;
	if(!(entry = vmem_resolve(r->mem_space->page_dir, r->begin)))
		return; // TODO Error?
	*entry |= PAGING_PAGE_WRITE;
}

static void copy_on_write(void *physical_page,
	mem_region_t *region, const size_t region_offset)
{
	mem_region_t *r;
	void *src;

	if(!(r = region->prev_shared))
		r = region->next_shared;
	if(!r)
		return;
	src = vmem_translate(region->mem_space->page_dir,
		region->begin + region_offset * PAGE_SIZE);
	memcpy(physical_page, src, PAGE_SIZE);
	if(region->prev_shared)
	{
		region->prev_shared->next_shared = region->next_shared;
		update_share(region->prev_shared);
	}
	if(region->next_shared)
	{
		region->next_shared->prev_shared = region->prev_shared;
		update_share(region->next_shared);
	}
	region->prev_shared = NULL;
	region->next_shared = NULL;
}

// TODO Map the whole region?
int mem_space_handle_page_fault(mem_space_t *space,
	void *ptr, const int error_code)
{
	mem_region_t *r;
	size_t region_offset;
	void *physical_page;

	if(!space || !ptr)
		return 0;
	ptr = ALIGN_DOWN(ptr, PAGE_SIZE);
	if(!(r = find_region(space->used_tree, ptr)))
		return 0;
	if(!(r->flags & MEM_REGION_FLAG_USER) || !(r->flags & MEM_REGION_FLAG_USER))
		return 0;
	if((error_code & PAGE_FAULT_WRITE) && !(r->flags & MEM_REGION_FLAG_WRITE))
		return 0;
	region_offset = (ptr - r->begin) / PAGE_SIZE;
	if(bitfield_get(r->use_bitfield, region_offset) == 0)
		return 0;
	if(!(physical_page = buddy_alloc_zero(0)))
		return 0;
	if(error_code & PAGE_FAULT_WRITE)
		copy_on_write(physical_page, r, region_offset);
	errno = 0;
	vmem_map(space->page_dir, physical_page, ptr, convert_flags(r->flags));
	if(errno)
	{
		buddy_free(physical_page);
		return 0;
	}
	return 1;
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
	// TODO Free gaps
	avl_tree_freeall(&space->used_tree, NULL);
	avl_tree_freeall(&space->free_tree, NULL);
	vmem_destroy(space->page_dir);
	cache_free(mem_space_cache, space);
}
