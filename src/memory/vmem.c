#include <memory/memory.h>

// TODO Handle shared memory (use free flag into page table entry)

__attribute__((hot))
static inline vmem_t new_vmem_obj(void)
{
	return buddy_alloc_zero(0);
}

__attribute__((hot))
vmem_t vmem_init(void)
{
	// TODO Default pages?
	return new_vmem_obj();
}

__attribute__((hot))
static void free_page_table(vmem_t table, const bool mem)
{
	if(mem)
	{
		for(size_t i = 0; i < PAGING_TABLE_SIZE; ++i)
		{
			if(!(table[i] & PAGING_PAGE_PRESENT))
				continue;

			buddy_free((void *) (table[i] & PAGING_ADDR_MASK));
		}
	}

	buddy_free(table);
}

__attribute__((hot))
static vmem_t clone_page_table(vmem_t from, const bool mem_dup)
{
	vmem_t v;
	if((v = new_vmem_obj()))
	{
		for(size_t i = 0; i < PAGING_TABLE_SIZE; ++i)
		{
			if(!(from[i] & PAGING_PAGE_PRESENT))
				continue;

			void *old_page = (void *) (from[i] & PAGING_ADDR_MASK);
			void *new_page = (mem_dup ? clone_page(old_page) : old_page);
			if(!new_page)
				goto fail;

			v[i] = ((uint32_t) new_page) | (from[i] & PAGING_FLAGS_MASK);
		}
	}

	return v;

fail:
	free_page_table(v, mem_dup);
	return NULL;
}

__attribute__((hot))
vmem_t vmem_clone(vmem_t vmem, const bool mem_dup)
{
	if(!vmem)
		return NULL;

	vmem_t v;
	if((v = vmem_init()))
	{
		for(size_t i = 0; i < PAGING_DIRECTORY_SIZE; ++i)
		{
			if(!(vmem[i] & PAGING_TABLE_PRESENT))
				continue;

			void *old_table_ptr = (void *) (vmem[i] & PAGING_ADDR_MASK);
			void *new_table_ptr;
			if(!(new_table_ptr = clone_page_table(old_table_ptr, mem_dup)))
				goto fail;

			v[i] = ((uint32_t) new_table_ptr) | (vmem[i] & PAGING_FLAGS_MASK);
		}
	}

	return v;

fail:
	vmem_free(v, false);
	return NULL;
}

__attribute__((hot))
void *vmem_translate(vmem_t vmem, void *ptr)
{
	const uintptr_t remain = (uintptr_t) ptr & 0xfff;
	const uintptr_t table = ((uintptr_t) ptr >> 12) & 0x3ff;
	const uintptr_t page = ((uintptr_t) ptr >> 22) & 0x3ff;

	if(!(vmem[table] & PAGING_TABLE_PRESENT))
		return NULL;

	vmem_t table_obj = (void *) (vmem[table] & PAGING_ADDR_MASK);
	if(!(table_obj[page] & PAGING_PAGE_PRESENT))
		return NULL;

	return (void *) ((table_obj[page] & PAGING_ADDR_MASK) | remain);
}

__attribute__((hot))
void *vmem_alloc_pages(vmem_t vmem, const size_t pages)
{
	if(!vmem)
		return NULL;

	void *ptr;
	if(!(ptr = buddy_alloc(pages)))
		return NULL;

	const uintptr_t table = ((uintptr_t) ptr >> 12) & 0x3ff;
	const uintptr_t page = ((uintptr_t) ptr >> 22) & 0x3ff;

	if(!(vmem[table] & PAGING_TABLE_PRESENT))
	{
		vmem_t table_ptr;
		if(!(table_ptr = new_vmem_obj()))
		{
			buddy_free(ptr);
			return (NULL);
		}

		// TODO vmem[table] with table
	}

	// TODO Set table entry
	(void) page;

	return ptr;
}

__attribute__((hot))
void vmem_free_pages(vmem_t vmem, const size_t pages, const bool mem_free)
{
	if(!vmem)
		return;

	// TODO
	(void) vmem;
	(void) pages;
	(void) mem_free;
}

__attribute__((hot))
void vmem_free(vmem_t vmem, const bool mem_free)
{
	if(!vmem)
		return;

	for(size_t i = 0; i < PAGING_DIRECTORY_SIZE; ++i)
	{
		if(!(vmem[i] & PAGING_TABLE_PRESENT))
			continue;

		free_page_table((void *) (vmem[i] & PAGING_ADDR_MASK), mem_free);
	}

	buddy_free(vmem);
}
