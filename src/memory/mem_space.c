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
 * If a region of memory is shared, the physical pages are not duplicated.
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
 *
 * When physical pages are allocated, only the page that the process tries to
 * access is being allocated, not the entire region. This allows to save memory
 * on stacks for example.
 */

// TODO Handle shared
// TODO Check gaps list order
// TODO Allocate only one page on access, not the entire region (except kernel stacks)

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
 * The default physical page, meant to be zero-ed and read only.
 */
void *default_page;

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
	if(!(default_page = buddy_alloc_zero(0)))
		PANIC("Failed to allocate memory space default page!", 0);
}

/*
 * Creates a memory gap with the given values for the given memory space. The
 * gap is not inserted in any data structure.
 */
static mem_gap_t *gap_create(mem_space_t *space,
	void *begin, const size_t pages)
{
	mem_gap_t *gap;

	debug_assert(sanity_check(space), "Invalid memory space");
	debug_assert(begin < begin + (pages * PAGE_SIZE), "Invalid gap");
	if(!(gap = cache_alloc(mem_gap_cache)))
		return NULL;
	gap->begin = begin;
	gap->pages = pages;
	gap->mem_space = space;
	gap->node.value = pages;
	return gap;
}

/*
 * Clones the given gap for the given destination space. The gap is not inserted
 * in any data structure.
 */
static mem_gap_t *gap_clone(mem_space_t *dest, mem_gap_t *g)
{
	mem_gap_t *new;

	if(!sanity_check(new = gap_create(dest, g->begin, g->pages)))
		return NULL;
	return new;
}

/*
 * Creates a gap of the specified size at the specified position and/or
 * extend/merge gaps around.
 */
static void extend_gaps(void *begin, const size_t pages)
{
	// TODO
	(void) begin;
	(void) pages;
}

/*
 * Creates the default memory gaps for the given memory space.
 */
static int init_gaps(mem_space_t *s)
{
	void *gap_begin;
	size_t gap_pages;
	mem_gap_t *gap;

	debug_assert(sanity_check(s), "Invalid memory space");
	gap_begin = (void *) 0x1000;
	gap_pages = (uintptr_t) KERNEL_BEGIN / PAGE_SIZE - 1;
	if(!(gap = gap_create(s, gap_begin, gap_pages)))
		return 0;
	list_insert_after(&s->gaps, s->gaps, &gap->list);
	avl_tree_insert(&s->free_tree, &gap->node, avl_val_cmp);
	// TODO Only expose a stub for the kernel
	gap_begin = mem_info.heap_begin;
	gap_pages = CEIL_DIVISION(mem_info.memory_end - mem_info.heap_begin,
		PAGE_SIZE);
	if(!(gap = gap_create(s, gap_begin, gap_pages)))
		return 0;
	list_insert_after(&s->gaps, s->gaps, &gap->list);
	avl_tree_insert(&s->free_tree, &gap->node, avl_val_cmp);
	return 1;
}

/*
 * Unlinks and frees the given gap.
 */
static void gap_free(mem_gap_t *gap)
{
	mem_space_t *mem_space;

	debug_assert(sanity_check(gap), "Invalid gap");
	mem_space = gap->mem_space;
	debug_assert(sanity_check(mem_space), "Invalid memory space");
	list_remove(&mem_space->gaps, &gap->list);
	avl_tree_remove(&mem_space->free_tree, &gap->node);
	cache_free(mem_gap_cache, gap);
}

/*
 * Removes the gap structure in the given list node.
 */
static void list_free_gap(list_head_t *l)
{
	debug_assert(sanity_check(l), "Invalid list");
	gap_free(CONTAINER_OF(l, mem_gap_t, list));
}

/*
 * Removes all gaps in the given list.
 * The pointer to the list has to be considered invalid after calling this
 * funciton.
 */
static void free_gaps(list_head_t *list)
{
	debug_assert(sanity_check(list), "Invalid list");
	list_foreach(list, list_free_gap);
}

/*
 * Creates a region for the given memory space. The region is not inserted in
 * any data structure.
 */
static mem_region_t *region_create(mem_space_t *space, const char flags,
	void *begin, const size_t pages, const size_t used_pages)
{
	mem_region_t *region;

	debug_assert(sanity_check(space), "Invalid memory space");
	if(!sanity_check(region = cache_alloc(mem_region_cache)))
		return NULL;
	region->mem_space = space;
	region->flags = flags;
	region->begin = begin;
	region->pages = pages;
	region->used_pages = used_pages;
	region->node.value = (avl_value_t) region->begin;
	return region;
}

/*
 * Clones the given region for the given destination space and links it to the
 * shared list.
 */
static mem_region_t *region_clone(mem_space_t *space, mem_region_t *r)
{
	mem_region_t *new;

	if(!sanity_check(new = region_create(space, r->flags, r->begin,
		r->pages, r->used_pages)))
		return NULL;
	// TODO Disable write here?
	list_insert_after(NULL, &r->shared_list, &new->shared_list);
	return new;
}

/*
 * Tells if the physical pages for the given region are shared with one or more
 * other memory spaces.
 */
static int region_is_shared(mem_region_t *region)
{
	debug_assert(sanity_check(region), "Invalid region");
	return (region->shared_list.prev || region->shared_list.next);
}

/*
 * Fills the region with mapping to the default page.
 */
static int region_phys_default(mem_region_t *r)
{
	void *page_dir;
	void *i;

	debug_assert(sanity_check(r), "Invalid region");
	page_dir = r->mem_space->page_dir;
	debug_assert(sanity_check(page_dir), "Invalid page directiory");
	i = r->begin;
	while(i < r->begin + (r->pages * PAGE_SIZE))
	{
		vmem_map(page_dir, default_page, i, PAGING_PAGE_USER);
		if(errno)
			return 0;
		i += PAGE_SIZE;
	}
	vmem_flush(page_dir);
	return 1;
}

/*
 * Frees physical pages for the given region.
 */
static void region_phys_free(mem_region_t *r)
{
	void *page_dir;
	void *i, *ptr;
	uint32_t *entry;

	debug_assert(sanity_check(r), "Invalid region");
	page_dir = r->mem_space->page_dir;
	debug_assert(sanity_check(page_dir), "Invalid page directiory");
	i = r->begin;
	while(i < r->begin + (r->pages * PAGE_SIZE))
	{
		entry = vmem_resolve(page_dir, i);
		debug_assert(sanity_check(entry), "Invalid paging entry");
		if(*entry & PAGING_PAGE_PRESENT)
		{
			ptr = (void *) (*entry & PAGING_ADDR_MASK);
			debug_assert(sanity_check(ptr), "Invalid physical page");
			buddy_free(ptr, 0);
			*entry = 0;
		}
		i += PAGE_SIZE;
	}
	vmem_flush(page_dir);
}

/*
 * Converts region space flags into paging flags.
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
 * Allocates physical pages for the given region.
 */
static int region_phys_alloc(mem_region_t *r)
{
	void *page_dir;
	void *i, *ptr;

	debug_assert(sanity_check(r), "Invalid region");
	page_dir = r->mem_space->page_dir;
	debug_assert(sanity_check(page_dir), "Invalid page directiory");
	i = r->begin;
	while(i < r->begin + (r->pages * PAGE_SIZE))
	{
		if(!(ptr = buddy_alloc_zero(0)))
			goto fail;
		vmem_map(page_dir, ptr, i, convert_flags(r->flags));
		if(errno)
			goto fail;
		i += PAGE_SIZE;
	}
	vmem_flush(page_dir);
	return 1;

fail:
	region_phys_free(r);
	return 0;
}

/*
 * Frees the given region, unlinks it and frees physical memory if needed.
 */
static void region_free(mem_region_t *region)
{
	mem_space_t *mem_space;

	debug_assert(sanity_check(region), "Invalid region");
	mem_space = region->mem_space;
	debug_assert(sanity_check(mem_space), "Invalid memory space");
	if(region_is_shared(region))
		list_remove(NULL, &region->shared_list);
	else
		region_phys_free(region);
	list_remove(&mem_space->regions, &region->list);
	avl_tree_remove(&mem_space->used_tree, &region->node);
	cache_free(mem_region_cache, region);
}

/*
 * Frees the region structure in the given list node.
 */
static void list_free_region(list_head_t *l)
{
	debug_assert(sanity_check(l), "Invalid list");
	region_free(CONTAINER_OF(l, mem_region_t, list));
}

/*
 * Frees all regions in the given list.
 * The pointer to the list has to be considered invalid after calling this
 * funciton.
 */
static void free_regions(list_head_t *list)
{
	debug_assert(sanity_check(list), "Invalid list");
	list_foreach(list, list_free_region);
}

/*
 * Clones the given regions to the given destination memory space.
 * Non userspace regions will not be cloned.
 */
static int clone_regions(mem_space_t *dest, list_head_t *regions)
{
	list_head_t *l;
	mem_region_t *r, *new;
	list_head_t *last = NULL;

	for(l = regions; l; l = l->next)
	{
		r = CONTAINER_OF(l, mem_region_t, list);
		if(!(r->flags & MEM_REGION_FLAG_USER))
		{
			// TODO Extend gaps around
			continue;
		}
		if(!sanity_check(new = region_clone(dest, r)))
		{
			free_regions(dest->regions);
			dest->regions = NULL;
			return 0;
		}
		list_insert_after(&dest->regions, last, &new->list);
		avl_tree_insert(&dest->used_tree, &new->node, avl_val_cmp);
		last = &new->list;
	}
	return 1;
}

/*
 * Clones the given gaps list to the given destination memory space.
 */
static int clone_gaps(mem_space_t *dest, list_head_t *gaps)
{
	list_head_t *l;
	mem_gap_t *g, *new;
	list_head_t *last = NULL;

	for(l = gaps; l; l = l->next)
	{
		g = CONTAINER_OF(l, mem_gap_t, list);
		if(!sanity_check(new = gap_clone(dest, g)))
		{
			free_gaps(dest->gaps);
			dest->gaps = NULL;
			return 0;
		}
		list_insert_after(&dest->gaps, last, &new->list);
		avl_tree_insert(&dest->free_tree, &new->node, avl_val_cmp);
		last = &new->list;
	}
	return 1;
}

/*
 * Disables write permission on the given region in the page directory link to
 * its memory space.
 */
static void regions_disable_write(mem_region_t *r)
{
	void *page_dir;
	void *ptr;
	size_t i;
	uint32_t *entry;

	debug_assert(sanity_check(r), "Invalid region");
	page_dir = r->mem_space->page_dir;
	debug_assert(sanity_check(page_dir), "Invalid page directory");
	ptr = r->begin;
	for(i = 0; i < r->pages; ++i)
	{
		if((entry = vmem_resolve(page_dir, ptr + (i * PAGE_SIZE))))
			*entry &= ~PAGING_PAGE_WRITE;
	}
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
	if(!sanity_check(s = cache_alloc(mem_space_cache)))
		return NULL;
	if(!init_gaps(s))
		goto fail;
	if(!sanity_check(s->page_dir = vmem_init()))
		goto fail;
	return s;

fail:
	free_gaps(s->gaps);
	cache_free(mem_space_cache, s);
	return NULL;
}

/*
 * Clones the given memory space. Physical pages are not cloned but will be when
 * accessed.
 * Non-userspace regions will not be cloned.
 */
mem_space_t *mem_space_clone(mem_space_t *space)
{
	mem_space_t *s;
	list_head_t *r;

	if(!space || !sanity_check(s = cache_alloc(mem_space_cache)))
		return NULL;
	spin_lock(&space->spinlock);
	if(!clone_regions(s, space->regions))
		goto fail;
	if(!clone_gaps(s, space->gaps))
		goto fail;
	for(r = space->regions; r; r = r->next)
		regions_disable_write(CONTAINER_OF(r, mem_region_t, list));
	if(!(s->page_dir = vmem_clone(space->page_dir)))
		goto fail;
	spin_unlock(&space->spinlock);
	return s;

fail:
	mem_space_destroy(s);
	spin_unlock(&space->spinlock);
	return NULL;
}

/*
 * Finds a gap large enough to fit the required number of pages.
 * If no gap large enough is found, `NULL` is returned.
 */
static avl_tree_t *find_gap(avl_tree_t *n, const size_t pages)
{
	if(!sanity_check(n) || pages == 0)
		return NULL;
	while(1)
	{
		if(n->left
			&& CONTAINER_OF(n->left, mem_gap_t, node)->pages > pages)
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
	debug_assert(pages <= g->pages, "Gap is too small");
	g->begin += pages * PAGE_SIZE;
	g->pages -= pages;
	if(g->pages <= 0)
	{
		gap_free(g);
		return;
	}
	avl_tree_remove(tree, gap);
	g->node.value = g->pages;
	avl_tree_insert(tree, gap, ptr_cmp);
}

/*
 * Tells whether the region should be preallocated or not.
 */
static inline int must_preallocate(const int flags)
{
	return (flags & MEM_REGION_FLAG_STACK) && !(flags & MEM_REGION_FLAG_USER);
}

/*
 * Maps the given region to identity.
 */
static void region_identity(mem_region_t *r)
{
	void *page_dir;
	void *i;

	debug_assert(sanity_check(r), "Invalid region");
	page_dir = r->mem_space->page_dir;
	debug_assert(sanity_check(page_dir), "Invalid page directiory");
	i = r->begin;
	while(i < r->begin + (r->pages * PAGE_SIZE))
	{
		vmem_identity(page_dir, i, convert_flags(r->flags));
		i += PAGE_SIZE;
	}
}

/*
 * Creates a region of the specified size with the specified flags. The function
 * will look for a gap large enough to fit the requested amount of pages, shrink
 * the gap and create the new region in that location.
 *
 * If the region is a kernel stack, it will be pre-allocated.
 */
static mem_region_t *mem_space_alloc_(mem_space_t *space,
	const size_t pages, const int flags)
{
	mem_region_t *r;
	avl_tree_t *gap;
	mem_gap_t *g;
	int prealloc;

	debug_assert(space, "Invalid memory space");
	debug_assert(!((flags & MEM_REGION_FLAG_IDENTITY)
		&& (flags & MEM_REGION_FLAG_STACK)), "Invalid flags");
	if(!sanity_check(gap = find_gap(space->free_tree, pages)))
	{
		errno = ENOMEM;
		return NULL;
	}
	g = CONTAINER_OF(gap, mem_gap_t, node);
	if(!sanity_check(r = region_create(space, flags, g->begin, pages, pages)))
	{
		errno = ENOMEM;
		return NULL;
	}
	if(r->flags & MEM_REGION_FLAG_IDENTITY)
		region_identity(r);
	prealloc = must_preallocate(r->flags);
	if((prealloc && !region_phys_alloc(r))
		|| (!prealloc && !region_phys_default(r)))
	{
		cache_free(mem_region_cache, r);
		return NULL;
	}
	list_insert_front(&space->regions, &r->list);
	avl_tree_insert(&space->used_tree, &r->node, avl_val_cmp);
	shrink_gap(&space->free_tree, gap, r->pages);
	return r;
}

/*
 * Allocates a region with the given number of pages and returns a pointer to
 * the beginning. If the requested allocation is a stack, then the pointer to
 * the top of the stack will be returned.
 */
ATTR_MALLOC
void *mem_space_alloc(mem_space_t *space, const size_t pages, const int flags)
{
	mem_region_t *r;
	void *ptr;

	if(!sanity_check(space))
		return NULL;
	if(pages == 0)
	{
		errno = EINVAL;
		return NULL;
	}
	spin_lock(&space->spinlock);
	if(!sanity_check(r = mem_space_alloc_(space, pages, flags)))
	{
		spin_unlock(&space->spinlock);
		return NULL;
	}
	debug_assert(r->begin, "Invalid region");
	if(flags & MEM_REGION_FLAG_STACK)
		ptr = r->begin + (r->pages * PAGE_SIZE) - 1;
	else
		ptr = r->begin;
	spin_unlock(&space->spinlock);
	return ptr;
}

/*
 * Allocates a region with the given number of pages at the specified location
 * and returns a pointer to the beginning. The allocation shall be exactly at
 * the given address and shall fail if not possible. On success, old region(s)
 * in place of the new one is/are replaced by the new one.
 */
ATTR_MALLOC
void *mem_space_alloc_fixed(mem_space_t *space, void *addr, size_t pages,
	int flags)
{
	if(!sanity_check(space))
		return NULL;
	if(!addr || pages == 0)
	{
		errno = EINVAL;
		return NULL;
	}
	// TODO
	(void) flags;
	return NULL;
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
		if(r->begin > ptr)
			n = n->left;
		else if(r->begin < ptr)
			n = n->right;
		else
			break;
	}
	if(!n)
		return NULL;
	if(ptr >= r->begin && ptr < r->begin + (r->pages * PAGE_SIZE))
		return r;
	return NULL;
}

/*
 * Shrinks, removes or splits the given region according to the given gap
 * pointer and size and creates/expands/merges gaps accordingly.
 */
static void region_split(mem_region_t *region, void *ptr, const size_t pages)
{
	/*if(pages == r->pages)
		region_free(r);
	else
		region_split(r, ptr, pages);*/
	// TODO
	extend_gaps(region->begin, pages);
	(void) ptr;
	(void) pages;
}

/*
 * Frees the given pages allocated in the given memory space.
 */
int mem_space_free(mem_space_t *space, void *ptr, const size_t pages)
{
	mem_region_t *r;

	if(!sanity_check(space))
		return 0;
	if(!ptr || ptr + pages * PAGE_SIZE <= ptr)
	{
		errno = -EINVAL;
		return 0;
	}
	spin_lock(&space->spinlock);
	if(!sanity_check(r = find_region(space->used_tree, ptr)))
		goto fail;
	if(pages > r->pages)
		goto fail;
	region_split(r, ptr, pages);
	spin_unlock(&space->spinlock);
	return 1;

fail:
	spin_unlock(&space->spinlock);
	return 0;
}

/*
 * Frees the given stack allocated in the given memory space.
 * The given pointer must be the same as returned by the allocation function.
 */
int mem_space_free_stack(mem_space_t *space, void *ptr)
{
	mem_region_t *r;

	if(!sanity_check(space))
		return 0;
	spin_lock(&space->spinlock);
	if(((uintptr_t) ptr & (PAGE_SIZE - 1)) != (PAGE_SIZE - 1))
		goto fail;
	if(!sanity_check(r = find_region(space->used_tree, ptr)))
		goto fail;
	if(!(r->flags & MEM_REGION_FLAG_STACK))
		goto fail;
	// TODO Free region and extend gaps
	spin_unlock(&space->spinlock);
	return 1;

fail:
	spin_unlock(&space->spinlock);
	return 0;
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

	if(!sanity_check(space) || !ptr)
		return 0;
	i = DOWN_ALIGN(ptr, PAGE_SIZE);
	end = DOWN_ALIGN(ptr + size, PAGE_SIZE);
	debug_assert(i <= end, "Invalid range");
	spin_lock(&space->spinlock);
	while(i < end)
	{
		if(!(r = find_region(space->used_tree, i))
			|| (write && !(r->flags & MEM_REGION_FLAG_WRITE)))
		{
			spin_unlock(&space->spinlock);
			return 0;
		}
		i += r->pages * PAGE_SIZE;
	}
	spin_unlock(&space->spinlock);
	return 1;
}

/*
 * Updates the write status of the given region.
 */
static void update_share(mem_region_t *r)
{
	int write;
	void *page_dir;
	size_t i;
	uint32_t *entry;

	debug_assert(sanity_check(r), "Invalid region");
	write = (r->flags & MEM_REGION_FLAG_WRITE)
		&& !(r->shared_list.prev || r->shared_list.next);
	page_dir = r->mem_space->page_dir;
	for(i = 0; i < r->pages; ++i)
	{
		entry = vmem_resolve(page_dir, r->begin + (i * PAGE_SIZE));
		debug_assert(sanity_check(entry), "Entry not found");
		if(write)
			*entry |= PAGING_PAGE_WRITE;
		else
			*entry &= ~PAGING_PAGE_WRITE;
	}
	vmem_flush(page_dir);
}

/*
 * On the shared list, removes the current region and calls `update_share` on
 * previous and next regions.
 */
static void update_near_regions(mem_region_t *region)
{
	list_head_t *prev, *next;

	debug_assert(sanity_check(region), "Invalid region");
	prev = region->shared_list.prev;
	next = region->shared_list.next;
	list_remove(NULL, &region->shared_list);
	if(prev)
		update_share(CONTAINER_OF(prev, mem_region_t, shared_list));
	if(next)
		update_share(CONTAINER_OF(next, mem_region_t, shared_list));
}

/*
 * Copies physical pages from region `src` to region `dest`.
 */
static void copy_pages(mem_region_t *dest, mem_region_t *src)
{
	size_t i;
	void *d, *s;

	debug_assert(sanity_check(dest) && sanity_check(src), "Invalid region");
	debug_assert(dest->begin == src->begin, "Incompatible regions");
	debug_assert(dest->pages == src->pages, "Incompatible regions");
	for(i = 0; i < dest->pages; ++i)
	{
		d = vmem_translate(dest->mem_space->page_dir, dest->begin);
		s = vmem_translate(src->mem_space->page_dir, src->begin);
		debug_assert(d && s, "Unallocated physical pages");
		memcpy(d, s, PAGE_SIZE);
	}
}

/*
 * Performs the Copy-On-Write operation if needed.
 * Duplicates physical pages for the given region with the same content as its
 * shared regions.
 * The region will be unlinked from the shared linked-list.
 *
 * On success, 1 is returned. 0 on fail.
 */
static int copy_on_write(mem_region_t *region)
{
	list_head_t *r;

	debug_assert(sanity_check(region), "Invalid region");
	if(!(r = region->shared_list.prev))
		r = region->shared_list.next;
	if(!sanity_check(r))
		return 0;
	// TODO If memory block cannot be found, use OOM-killer
	if(!region_phys_alloc(region))
		return 0;
	copy_pages(region, CONTAINER_OF(r, mem_region_t, list));
	update_near_regions(region);
	return 1;
}

/*
 * Handles a page fault. This function returns 1 if the new page was correctly
 * allocated and 0 if the process should be killed by a signal or if the
 * kernel should panic.
 *
 * The function will return 0 if:
 * - The page fault was not caused by a write operation
 * - The region for the given pointer cannot be found
 * - The the region isn't writable
 * - The page fault was caused by a userspace operation and the region is not
 * in userspace
 * - The memory could not have been allocated
 */
int mem_space_handle_page_fault(mem_space_t *space,
	void *ptr, const int error_code)
{
	mem_region_t *r;

	if(!sanity_check(space) || !ptr)
		return 0;
	if(!(error_code & PAGE_FAULT_WRITE))
		return 0;
	spin_lock(&space->spinlock);
	ptr = DOWN_ALIGN(ptr, PAGE_SIZE);
	if(!sanity_check(r = find_region(space->used_tree, ptr)))
		goto fail;
	if(!(r->flags & MEM_REGION_FLAG_WRITE))
		goto fail;
	if((error_code & PAGE_FAULT_USER) && !(r->flags & MEM_REGION_FLAG_USER))
		goto fail;
	if(r->shared_list.prev || r->shared_list.next)
	{
		spin_unlock(&space->spinlock);
		return copy_on_write(r);
	}
	spin_unlock(&space->spinlock);
	return region_phys_alloc(r);

fail:
	spin_unlock(&space->spinlock);
	return 0;
}

/*
 * Destroyes the given memory space and regions/gaps in it and frees non-shared
 * physical pages.
 */
void mem_space_destroy(mem_space_t *space)
{
	if(!sanity_check(space))
		return;
	spin_lock(&space->spinlock);
	free_regions(space->regions);
	free_gaps(space->gaps);
	vmem_destroy(space->page_dir);
	cache_free(mem_space_cache, space);
	spin_unlock(&space->spinlock);
}
