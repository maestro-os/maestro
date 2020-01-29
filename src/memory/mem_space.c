#include <memory/memory.h>
#include <kernel.h>

/*
 * This file handles memory spaces handling, used to divide processes' memory
 * space.
 *
 * The mem_space provides an architecture-independant interface for handling
 * processes memory. It allows to allocate virtual memory space and to use the
 * COW (Copy-On-Write) feature of the kernel which avoids useless copies of
 * memory.
 *
 * When a copy of a memory space is made, the write access to the physical pages
 * is disabled to make the kernel able to detect when a process tries to access
 * the memory.
 * When trying to write to the memory, the physical page will be cloned and the
 * process will have its virtual page remapped to the newly allocated page.
 *
 * - Memory regions (structure `mem_region`) tells which parts of the virtual
 * space is being used by allocated memory.
 * - Memory gaps (structure `mem_gap`) tells which parts of the virtual space
 * is ready to be used for further allocations.
 *
 * Gaps determine the locations where virtual memory can be allocated.
 * When setting default gaps, the kernel shall create a first gap from the
 * second page of memory to the beginning of the kernel stub, and a second gap
 * from the end of the kernel stub to the before last page of the available
 * memory.
 * The reason for not allowing allocations of the first page is because the
 * `NULL` pointer is located at the beginning of it and needs not to be
 * accessible.
 * The kernel stub must not be allocated neither because the process must keep
 * access to it in order to be able to perform system calls.
 * The last page is not included to prevent overflows.
 *
 * When a region of memory is allocated, the physical memory is not allocated
 * directly, except for kernelspace stacks which need to be allocated directly
 * because the kernel is using the Page Fault exception to detect access to
 * memory that was not yet allocated. However if the kernel stack was not
 * pre-allocated, the CPU shall trigger a Double Fault exception which shall
 * lead to a Triple Fault and reset the system.
 */

// TODO Spinlock
// TODO Check if linked lists are useful
// TODO Use pages allocator instead of buddy allocator?
// TODO Allocate whole regions at once instead of 1 page?

/*
 * The cache for the `mem_space` structure.
 */
static cache_t *mem_space_cache;
/*
 * The cache for the `mem_region` structure.
 */
static cache_t *mem_region_cache;
/*
 * The cache for the `mem_gap` structure.
 */
static cache_t *mem_gap_cache;

/*
 * Initializes caches.
 */
static void global_init(void)
{
	if(!(mem_space_cache = cache_create("mem_space", sizeof(mem_space_t), 64,
		bzero, NULL)))
		PANIC("Failed to initialize mem_space cache!", 0);
	if(!(mem_region_cache = cache_create("mem_region", sizeof(mem_region_t), 64,
		bzero, NULL)))
		PANIC("Failed to initialize mem_region cache!", 0);
	if(!(mem_gap_cache = cache_create("mem_gap", sizeof(mem_gap_t), 64,
		bzero, NULL)))
		PANIC("Failed to initialize mem_gap cache!", 0);
}

/*
 * Creates the default memory gaps for the given memory space.
 */
static int init_gaps(mem_space_t *s)
{
	if(!(s->gaps = cache_alloc(mem_gap_cache)))
		return 0;
	s->gaps->begin = (void *) 0x1000;
	s->gaps->pages = 0xffffe;
	// TODO Kernel code/syscall stub must not be inside a gap
	errno = 0;
	s->gaps->node.value = s->gaps->pages;
	avl_tree_insert(&s->free_tree, &s->gaps->node, avl_val_cmp);
	if(errno)
	{
		cache_free(mem_gap_cache, s->gaps);
		return 0;
	}
	return 1;
}

/*
 * Creates a new memory space.
 */
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
	if(!init_gaps(s))
		goto fail;
	if(!(s->page_dir = vmem_init()))
		goto fail;
	return s;

fail:
	cache_free(mem_gap_cache, s->gaps); // TODO Might be several gaps
	cache_free(mem_space_cache, s);
	return NULL;
}

/*
 * Clones the given region for the given destination space.
 */
static mem_region_t *clone_region(mem_space_t *space, mem_region_t *r)
{
	mem_region_t *new;

	if(!(new = cache_alloc(mem_region_cache)))
		return NULL;
	new->mem_space = space;
	new->flags = r->flags;
	new->begin = r->begin;
	new->pages = r->pages;
	new->used_pages = r->used_pages;
	if((new->next_shared = r->next_shared))
		r->next_shared->prev_shared = new;
	if((new->prev_shared = r))
		r->next_shared = new;
	return new;
}

/*
 * Frees the given region, unlinks it from shared linked list and frees physical
 * memory if needed.
 */
static void region_free(mem_region_t *region)
{
	if(!region->prev_shared && !region->next_shared)
		pages_free(region->begin, region->pages * PAGE_SIZE);
	else
	{
		if(region->prev_shared)
			region->prev_shared->next_shared = region->next_shared;
		if(region->next_shared)
			region->next_shared->prev_shared = region->prev_shared;
	}
	cache_free(mem_region_cache, region);
}

/*
 * Frees the given region list.
 */
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

/*
 * Clones the given regions to the given destination memory space.
 */
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

/*
 * TODO
 */
static void gap_free(mem_gap_t *gap)
{
	// TODO
	(void) gap;
}

/*
 * Frees the given gaps list.
 */
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

/*
 * Clones the given gaps list to the given destination memory space.
 */
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

// TODO Remove, build trees during cloning of regions and gaps
static int build_trees(mem_space_t *space)
{
	mem_region_t *r;
	mem_gap_t *g;

	r = space->regions;
	errno = 0;
	while(r)
	{
		r->node.value = (avl_value_t) r->begin;
		avl_tree_insert(&space->used_tree, &r->node, avl_val_cmp);
		if(errno)
			return 0;
		r = r->next;
	}
	g = space->gaps;
	while(g)
	{
		g->node.value = (avl_value_t) g->pages;
		avl_tree_insert(&space->free_tree, &g->node, avl_val_cmp);
		if(errno)
			return 0;
		g = g->next;
	}
	return 1;
}

/*
 * Disables writing on the given region and x86 paging directory.
 */
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

/*
 * Converts region space flags into x86 paging flags.
 */
static int convert_flags(const int reg_flags)
{
	int flags = 0;

	if(reg_flags & MEM_REGION_FLAG_WRITE)
		flags |= PAGING_PAGE_WRITE;
	if(reg_flags & MEM_REGION_FLAG_USER)
		flags |= PAGING_PAGE_USER;
	return flags;
}

/*
 * Preallocates the kernel stack associated with the given space and region.
 */
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

/*
 * Clones the given memory space. Physical pages are not cloned but will be when
 * accessed.
 */
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

/*
 * Finds a gap large enough to fit the required number of pages.
 * If no gap large enough is found, `NULL` is returned.
 */
static avl_tree_t *find_gap(avl_tree_t *n, const size_t pages)
{
	if(!n || pages == 0)
		return NULL;
	while(1)
	{
		if(n->left
			&& CONTAINER_OF(n->left, mem_gap_t, node)->pages >= pages)
			n = n->left;
		else if(n->right
			&& CONTAINER_OF(n->right, mem_gap_t, node)->pages < pages)
			n = n->right;
		else
			break;
	}
	return n;
}

/*
 * Shrinks the given gap to the given amount of pages. The pointer to the
 * beginning of the gap will increase to that amount of pages and the location
 * of the gap in its tree will be updated.
 */
static void shrink_gap(avl_tree_t **tree, avl_tree_t *gap, const size_t pages)
{
	mem_gap_t *g;

	if(!gap || pages == 0)
		return;
	g = CONTAINER_OF(gap, mem_gap_t, node);
	// TODO Error if pages > gap->pages? (shouldn't be possible)
	if(g->pages <= pages)
	{
		if(g->prev)
			g->prev->next = g->next;
		if(g->next)
			g->next->prev = g->prev;
		avl_tree_remove(tree, gap);
		cache_free(mem_gap_cache, g);
		return;
	}
	g->begin += pages * PAGE_SIZE;
	g->pages -= pages;
	// TODO Remove and re-insert the node in the tree
}

/*
 * Creates a region of the specified size with the specified flags. The function
 * will look for a gap large enough to fit the requested amount of pages, shrink
 * the gap and create the new region in that location.
 *
 * If the region is a kernel stack, it will be pre-allocated.
 */
static mem_region_t *region_create(mem_space_t *space,
	const size_t pages, const int flags)
{
	mem_region_t *r;
	avl_tree_t *gap;

	if(pages == 0)
		return NULL;
	if(!(r = cache_alloc(mem_region_cache)))
		return NULL;
	if(!(gap = find_gap(space->free_tree, pages)))
	{
		cache_free(mem_region_cache, r);
		return NULL;
	}
	r->mem_space = space;
	r->flags = flags;
	r->begin = CONTAINER_OF(gap, mem_gap_t, node)->begin;
	r->pages = pages;
	r->used_pages = r->pages;
	if(!(flags & MEM_REGION_FLAG_USER) && (flags & MEM_REGION_FLAG_STACK)
		&& !preallocate_kernel_stack(space, r))
	{
		cache_free(mem_region_cache, r);
		return NULL;
	}
	errno = 0;
	r->node.value = (avl_value_t) r->begin;
	avl_tree_insert(&space->used_tree, &r->node, avl_val_cmp);
	if(errno)
	{
		// TODO If preallocated kernel_stack, free it
		cache_free(mem_region_cache, r);
		return NULL;
	}
	shrink_gap(&space->free_tree, gap, pages);
	return r;
}

/*
 * Allocates a region with the given number of pages and returns a pointer to
 * the beginning. If the requested allocation is a stack, then the pointer to
 * the top of the stack will be returned.
 */
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

/*
 * Finds the regions containing the given pointer.
 * If no region is found, `NULL` is returned.
 */
static mem_region_t *find_region(avl_tree_t *n, void *ptr)
{
	mem_region_t *r = NULL;

	if(!ptr)
		return NULL;
	while(n)
	{
		r = CONTAINER_OF(n, mem_region_t, node);
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

/*
 * Frees the given pages allocated in the given memory space.
 */
void mem_space_free(mem_space_t *space, void *ptr, const size_t pages)
{
	if(!space || !ptr || pages == 0)
		return;
	// TODO Find region using tree
	// TODO If the whole region is to be freed, free it
	// TODO If only a part of the region is to be freed, spilt into new regions
	// TODO Get and extend near gap if needed
	// TODO Merge gaps if needed
	// TODO Update page directory
}

/*
 * Frees the given stack allocated in the given memory space.
 * The given pointer must be the same as returned by the allocation function.
 */
void mem_space_free_stack(mem_space_t *space, void *stack)
{
	if(!space || !stack)
		return;
	// TODO Find region using tree and free it
	// TODO Get and extend near gaps
	// TODO Merge gaps if needed
	// TODO Update page directory
}

/*
 * Checks if the given portion of memory is accessible in the given memory
 * space. `write` tells whether the portion should writable or not.
 */
int mem_space_can_access(mem_space_t *space, const void *ptr, const size_t size,
	const int write)
{
	void *i, *end;
	mem_region_t *r;

	if(!space || !ptr)
		return 0;
	i = DOWN_ALIGN(ptr, PAGE_SIZE);
	end = DOWN_ALIGN(ptr + size, PAGE_SIZE);
	while(i < end)
	{
		if(!(r = find_region(space->used_tree, i)))
			return 0;
		if(write && !(r->flags & MEM_REGION_FLAG_WRITE))
			return 0;
		i += r->pages * PAGE_SIZE;
	}
	return 1;
}

/*
 * TODO
 */
static void update_share(mem_region_t *r)
{
	uint32_t *entry;

	if(r->prev_shared || r->next_shared || !(r->flags & MEM_REGION_FLAG_WRITE))
		return;
	if(!(entry = vmem_resolve(r->mem_space->page_dir, r->begin)))
		return; // TODO Error?
	*entry |= PAGING_PAGE_WRITE;
}

/*
 * Performs the Copy-On-Write operation. Copies the content of the given
 * region at the given offset (in pages) to the given physical page.
 * The region will be unlinked from the shared linked-list.
 */
static int copy_on_write(mem_region_t *region)
{
	mem_region_t *r;
	void *dest, *src;

	if(!(r = region->prev_shared))
		r = region->next_shared;
	if(!r)
		return 0;
	// TODO If linear block cannot be found, try to use a non-linear block
	// TODO If even non-linear block cannot be found, use OOM-killer
	if(!(dest = pages_alloc(region->pages)))
		return 0;
	src = vmem_translate(region->mem_space->page_dir, region->begin);
	memcpy(dest, src, region->pages * PAGE_SIZE);
	errno = 0;
	vmem_map_range(region->mem_space->page_dir, dest, region->begin,
		region->pages, convert_flags(r->flags));
	if(errno)
	{
		pages_free(dest, 0);
		return 0;
	}
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
	return 1;
}

// TODO Map the whole region?
/*
 * Handles a page fault. This function returns `1` if the new page was correctly
 * allocated and `0` if the process should be killed by a signal or if the
 * kernel should panic.
 *
 * The function will return `0` if:
 * - The page fault was not caused by a write operation
 * - The region for the given pointer cannot be found
 * - The the region isn't writable
 * - The page fault was caused by a userspace operation and the region is not
 * in userspace
 * - An error happened while mapping the allocated page
 */
int mem_space_handle_page_fault(mem_space_t *space,
	void *ptr, const int error_code)
{
	mem_region_t *r;

	if(!space || !ptr)
		return 0;
	if(!(error_code & PAGE_FAULT_WRITE))
		return 0;
	ptr = DOWN_ALIGN(ptr, PAGE_SIZE);
	if(!(r = find_region(space->used_tree, ptr)))
		return 0;
	if(!(r->flags & MEM_REGION_FLAG_WRITE))
		return 0;
	if((error_code & PAGE_FAULT_USER) && !(r->flags & MEM_REGION_FLAG_USER))
		return 0;
	return copy_on_write(r);
}

/*
 * Destroyes the given memory space. Destroyes not shared memory regions.
 */
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
	vmem_destroy(space->page_dir);
	cache_free(mem_space_cache, space);
}
