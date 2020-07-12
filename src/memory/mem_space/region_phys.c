#include <kernel.h>
#include <memory/mem_space/mem_space.h>
#include <memory/memory.h>
#include <debug/debug.h>

#include <libc/errno.h>

/*
 * This file handles physical allocations for regions.
 */

/*
 * The default physical page, meant to be zero-ed and read only.
 */
static void *default_page;

/*
 * Allocates the default physical page.
 */
static void phys_global_init(void)
{
	default_page = buddy_alloc_zero(0);
	if(!default_page)
		PANIC("Memory spaces initialization failed!", 0);
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
 * Fills the region with mapping to the default page.
 */
int region_phys_default(mem_region_t *r)
{
	static int init = 0;
	void *page_dir;
	void *i;

	if(unlikely(!init))
	{
		phys_global_init();
		init = 1;
	}
	debug_assert(sanity_check(r), "Invalid region");
	page_dir = r->mem_space->page_dir;
	debug_assert(sanity_check(page_dir), "Invalid page directiory");
	i = r->begin;
	while(i < r->begin + (r->pages * PAGE_SIZE))
	{
		vmem_map(page_dir, default_page, i, PAGING_PAGE_USER);
		if(errno)
			goto fail;
		i += PAGE_SIZE;
	}
	vmem_flush(page_dir);
	return 1;

fail:
	vmem_unmap_range(page_dir, r->begin, r->pages);
	vmem_flush(page_dir);
	return 0;
}

/*
 * Maps the given region to identity.
 */
void region_phys_identity(mem_region_t *r)
{
	void *page_dir;
	void *i;

	debug_assert(sanity_check(r), "Invalid region");
	page_dir = r->mem_space->page_dir;
	debug_assert(sanity_check(page_dir), "Invalid page directiory");
	i = r->begin;
	while(i < r->begin + (r->pages * PAGE_SIZE))
	{
		// TODO Use `vmem_identity_range`
		vmem_identity(page_dir, i, convert_flags(r->flags));
		i += PAGE_SIZE;
	}
}

/*
 * Allocates physical pages for the given region.
 */
int region_phys_alloc(mem_region_t *r)
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
 * Frees physical pages for the given region.
 */
void region_phys_free(mem_region_t *r)
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
