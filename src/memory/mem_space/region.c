#include <kernel.h>
#include <memory/mem_space/mem_space.h>
#include <memory/slab/slab.h>
#include <debug/debug.h>

/*
 * This file handles regions for memory space.
 *
 * A region represents an allocated zone of the virtual memory that can be
 * freed. A region can be shared between several spaces and thus sharing the
 * same physical page. This, among others, allow to implement COW
 * (Copy-On-Write), which is lazy allocation of physical pages after forking a
 * process.
 * Every region has lazy allocation of the physical page, thus allocations
 * virtually cannot fail. If the system lacks memory it will swap some memory
 * pages and, under extreme conditions, use the OOM (Out Of Memory) killer to
 * kill non important processes and recover some memory to fullfill the
 * allocation when needed.
 */

/*
 * The cache for the `mem_region` structure.
 */
static cache_t *mem_region_cache;

/*
 * Initializes regions.
 */
static void regions_global_init(void)
{
	mem_region_cache = cache_create("mem_region", sizeof(mem_region_t), 64,
		bzero, NULL);
	if(!mem_region_cache)
		PANIC("Memory spaces initialization failed!", 0);
}

/*
 * Creates a region for the given memory space. The region is not inserted in
 * any data structure.
 */
mem_region_t *region_create(mem_space_t *space, const char flags, void *begin,
	const size_t pages, const size_t used_pages)
{
	static int init = 0;
	mem_region_t *region;

	if(unlikely(!init))
	{
		regions_global_init();
		init = 1;
	}
	debug_assert(sanity_check(space), "Invalid memory space");
	ASSERT_RANGE(begin, pages);
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
mem_region_t *region_clone(mem_space_t *space, mem_region_t *r)
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
int region_is_shared(mem_region_t *region)
{
	debug_assert(sanity_check(region), "Invalid region");
	return (region->shared_list.prev || region->shared_list.next);
}

/*
 * Frees the given region, unlinks it and frees physical memory if needed.
 */
void region_free(mem_region_t *region)
{
	mem_space_t *mem_space;

	debug_assert(sanity_check(region), "Invalid region");
	mem_space = region->mem_space;
	debug_assert(sanity_check(mem_space), "Invalid memory space");
	if(region_is_shared(region))
		list_remove(NULL, &region->shared_list);
	else
		region_phys_free(region); // TODO Do not call when identity and not a kernel stack (create a specific flag?)
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
void regions_free(list_head_t *list)
{
	debug_assert(sanity_check(list), "Invalid list");
	list_foreach(list, list_free_region);
}

/*
 * Clones the given regions to the given destination memory space.
 * Non userspace regions will not be cloned.
 *
 * On success, returns 1. On fail, returns 0.
 */
int regions_clone(mem_space_t *dest, list_head_t *regions)
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
			regions_free(dest->regions);
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
 * Disables write permission on the given region in the page directory link to
 * its memory space.
 */
void regions_disable_write(mem_region_t *r)
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
 * Finds the regions containing the given pointer.
 * If no region is found, `NULL` is returned.
 */
mem_region_t *region_find(avl_tree_t *n, void *ptr)
{
	mem_region_t *r = NULL;

	if(!ptr)
		return NULL;
	while(n)
	{
		r = CONTAINER_OF(n, mem_region_t, node);
		if(ptr >= r->begin && ptr < r->begin + (r->pages * PAGE_SIZE))
			return r;
		if(r->begin > ptr)
			n = n->left;
		else if(r->begin < ptr)
			n = n->right;
		else
			break;
	}
	return NULL;
}

/*
 * Creates a hole in the given region according to the given range.
 * The given region and range bound by `addr` and `pages` must overlap.
 * Region `r` shall be invalid after calling this function.
 *
 * On success, returns 1. On fail, returns 0.
 */
int region_split(mem_region_t *r, void *addr, const size_t pages)
{
	debug_assert(addr + PAGE_SIZE * pages >= r->begin + PAGE_SIZE * r->pages
		&& addr <= r->begin, "Region and range must overlap");
	ASSERT_RANGE(addr, pages);
	if(addr > r->begin)
	{
		// TODO Keep a region before
	}
	if(addr + PAGE_SIZE * pages <= r->begin + PAGE_SIZE * r->pages)
	{
		// TODO Keep a region after
	}
	region_free(r);
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
void regions_update_near(mem_region_t *region)
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
void region_copy_pages(mem_region_t *dest, mem_region_t *src)
{
	size_t i;
	void *d, *s;

	debug_assert(sanity_check(dest) && sanity_check(src), "Invalid region");
	debug_assert(dest->begin == src->begin
		&& dest->pages == src->pages, "Incompatible regions");
	for(i = 0; i < dest->pages; ++i)
	{
		d = vmem_translate(dest->mem_space->page_dir, dest->begin);
		s = vmem_translate(src->mem_space->page_dir, src->begin);
		debug_assert(d && s, "Unallocated physical pages");
		memcpy(d, s, PAGE_SIZE);
	}
}
