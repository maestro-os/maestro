#include <memory/mem_space/mem_space.h>
#include <memory/slab/slab.h>
#include <kernel.h>
#include <debug/debug.h>

#include <libc/errno.h>

#define KERNEL_STACK_FLAGS\
	MEM_REGION_FLAG_WRITE | MEM_REGION_FLAG_STACK | MEM_REGION_FLAG_IDENTITY

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
 * from the end of the kernel stub to the page before last page of the available
 * memory.
 * The reason for not allowing allocations of the first page is because the
 * `NULL` pointer is located at the beginning of it and needs not to be
 * accessible.
 * The kernel stub must not be allocated neither because the process must keep
 * access to it in order to be able to perform system calls.
 * The last page is not included to prevent overflows.
 *
 * When a region of memory is allocated, the physical memory is not allocated
 * directly, except for kernelspace stacks because the kernel is using the Page
 * Fault exception to detect access to memory that was not yet allocated.
 * However if the kernel stack was not pre-allocated, the CPU shall trigger a
 * Double Fault exception which shall lead to a Triple Fault and reset the
 * system.
 *
 * When physical pages are allocated, only the page that the process tries to
 * access is being allocated, not the entire region. This allows to save memory
 * on stacks for example.
 */

// TODO Handle shared
// TODO Check gaps list order
// TODO Allocate only one page on access, not the entire region
// TODO Handle file mapping
// TODO Empty page after stack to make segfault on overflow

/*
 * The cache for the `mem_space` structure.
 */
static cache_t *mem_space_cache;

/*
 * Initializes memory spaces.
 */
static void mem_space_global_init(void)
{
	mem_space_cache = cache_create("mem_space", sizeof(mem_space_t), 64,
		bzero, NULL);
	if(!mem_space_cache)
		PANIC("Memory spaces initialization failed!", 0);
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
		mem_space_global_init();
		init = 1;
	}
	if(!sanity_check(s = cache_alloc(mem_space_cache)))
		return NULL;
	if(!gaps_init(s))
		goto fail;
	if(!sanity_check(s->page_dir = vmem_init()))
		goto fail;
	return s;

fail:
	gaps_free(s->gaps);
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
	if(!regions_clone(s, space->regions))
		goto fail;
	if(!gaps_clone(s, space->gaps))
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
 * Removes/shrinks region/gap in the given interval of memory to make it empty.
 *
 * On success, returns 1. On fail, returns 0.
 */
static int mem_space_crush(mem_space_t *space, void *addr, const size_t pages)
{
	//mem_gap_t *g;
	mem_region_t *r;

	// TODO Find a way to get gaps from address
	/*if(sanity_check(g = find_gap(space->free_tree, addr)))
	{
		// TODO Remove/shrink region gap
	}*/
	if(sanity_check(r = region_find(space->used_tree, addr)))
	{
		if(!region_split(r, addr, pages))
		{
			// TODO Cancel gap removal
			return 0;
		}
	}
	return 1;
}

/*
 * Creates a region of the specified size with the specified flags at the
 * specified virtual address.
 * If the crush flag is enabled, gaps or regions at this location will be cut
 * or removed.
 */
static mem_region_t *mem_space_alloc__(mem_space_t *space, void *addr,
	const size_t pages, const int flags, const int crush)
{
	mem_region_t *r;

	if(!sanity_check(r = region_create(space, flags, addr, pages, pages)))
	{
		errno = ENOMEM;
		return NULL;
	}
	if(r->flags & MEM_REGION_FLAG_IDENTITY)
		region_phys_identity(r);
	if(!region_phys_default(r)
		|| (crush && !mem_space_crush(space, addr, pages)))
	{
		region_free(r);
		return NULL;
	}
	list_insert_front(&space->regions, &r->list);
	avl_tree_insert(&space->used_tree, &r->node, avl_val_cmp);
	return r;
}

/*
 * Creates a region of the specified size with the specified flags. The function
 * will look for a gap large enough to fit the requested amount of pages, shrink
 * the gap and create the new region in that location.
 */
static mem_region_t *mem_space_alloc_(mem_space_t *space, const size_t pages,
	const int flags)
{
	avl_tree_t *gap;
	mem_gap_t *g;
	mem_region_t *r;

	debug_assert(space, "Invalid memory space");
	if(!sanity_check(gap = gap_find(space->free_tree, pages)))
	{
		errno = ENOMEM;
		return NULL;
	}
	g = CONTAINER_OF(gap, mem_gap_t, node);
	if(!sanity_check(r = mem_space_alloc__(space, g->begin, pages, flags, 0)))
		return NULL;
	gap_shrink(&space->free_tree, gap, r->pages);
	return r;
}

/*
 * Allocates a region with the given number of pages and returns a pointer to
 * the beginning. If the requested allocation is a stack, then the pointer to
 * the top of the stack will be returned.
 *
 * If a kernel stack is requested, the function shall fail with errno EINVAL.
 * To allocate a stack, `mem_space_alloc_kernel_stack` shall be used.
 */
ATTR_MALLOC
void *mem_space_alloc(mem_space_t *space, const size_t pages, const int flags)
{
	mem_region_t *r;
	void *ptr;

	if(!sanity_check(space))
		return NULL;
	if(pages == 0
		|| ((flags & MEM_REGION_FLAG_STACK) && !(flags & MEM_REGION_FLAG_USER)))
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
	const int flags)
{
	mem_region_t *r;
	void *ptr;

	if(!sanity_check(space))
		return NULL;
	if(!addr || pages == 0)
	{
		errno = EINVAL;
		return NULL;
	}
	spin_lock(&space->spinlock);
	if(!sanity_check(r = mem_space_alloc__(space, addr,  pages, flags, 1)))
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
 * Allocates a kernel stack with the given buddy allocator order as size.
 * Kernel stacks are preallocated and identity mapped.
 */
ATTR_MALLOC
void *mem_space_alloc_kernel_stack(mem_space_t *space, const size_t buddy_order)
{
	void *ptr;

	// TODO Zero?
	if(!(ptr = buddy_alloc(buddy_order)))
	{
		errno = ENOMEM;
		return NULL;
	}
	if(!mem_space_alloc_fixed(space, ptr, FRAME_SIZE(buddy_order),
		KERNEL_STACK_FLAGS))
	{
		buddy_free(ptr, buddy_order);
		return NULL;
	}
	return ptr;
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
	if(!sanity_check(r = region_find(space->used_tree, ptr)))
		goto fail;
	if(pages > r->pages) // TODO Ignore and clamp to region size?
		goto fail;
	if(!gap_extend(&space->free_tree , ptr, pages)
		|| !region_split(r, ptr, pages))
		goto fail; // TODO Remove created gap
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
	if(!sanity_check(r = region_find(space->used_tree, ptr)))
		goto fail;
	if(!(r->flags & MEM_REGION_FLAG_STACK))
		goto fail;
	// TODO extend_gaps and region_split
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
		if(!(r = region_find(space->used_tree, i))
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
 * Copies `n` bytes from the given `src` address in the given memory space to
 * the given `dst` address in the current context.
 * If a portion of the memory space used in the copy is unreachable, the
 * behaviour is undefined.
 */
void mem_space_copy_from(mem_space_t *space, void *dst, const void *src,
	const size_t n)
{
	size_t i;
	void *ptr;
	size_t size;

	if(!sanity_check(space) || !sanity_check(dst) || !src)
		return;
	spin_lock(&space->spinlock);
	i = 0;
	while(i < n)
	{
		ptr = vmem_translate(space->page_dir, src + i);
		debug_assert(ptr, "Invalid memory");
		size = MIN(n - i, PAGE_SIZE - (i % PAGE_SIZE));
		memcpy(dst + i, ptr, size);
		i += size;
	}
	spin_unlock(&space->spinlock);
}

/*
 * Copies `n` bytes from the given `src` address in the current context to
 * the given `dst` address in the given memory space.
 * If a portion of the memory space used in the copy is unreachable, the
 * behaviour is undefined.
 * The function doesn't check write access to the destination page and will
 * try to write on it anyways.
 */
void mem_space_copy_to(mem_space_t *space, void *dst, const void *src,
	const size_t n)
{
	size_t i;
	void *ptr;
	size_t size;

	if(!sanity_check(space) || !sanity_check(dst) || !src)
		return;
	spin_lock(&space->spinlock);
	i = 0;
	while(i < n)
	{
		ptr = vmem_translate(space->page_dir, src + i);
		debug_assert(ptr, "Invalid memory");
		size = MIN(n - i, PAGE_SIZE - (i % PAGE_SIZE));
		memcpy(ptr, dst + i, size);
		i += size;
	}
	spin_unlock(&space->spinlock);
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
	region_copy_pages(region, CONTAINER_OF(r, mem_region_t, list));
	regions_update_near(region);
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
int mem_space_handle_page_fault(mem_space_t *space, void *ptr,
	const int error_code)
{
	mem_region_t *r;

	if(!sanity_check(space) || !ptr)
		return 0;
	if(!(error_code & PAGE_FAULT_WRITE))
		return 0;
	spin_lock(&space->spinlock);
	ptr = DOWN_ALIGN(ptr, PAGE_SIZE);
	if(!sanity_check(r = region_find(space->used_tree, ptr)))
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
	regions_free(space->regions);
	gaps_free(space->gaps);
	vmem_destroy(space->page_dir);
	cache_free(mem_space_cache, space);
	spin_unlock(&space->spinlock);
}
