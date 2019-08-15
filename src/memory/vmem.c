#include <memory/memory.h>

// TODO Handle shared memory (use free flag into page table entry)

vmem_t kernel_vmem;

__attribute__((hot))
static inline vmem_t new_vmem_obj(void)
{
	return buddy_alloc_zero(0);
}

// TODO Optimize
__attribute__((hot))
void vmem_kernel(void)
{
	size_t i, j;
	vmem_t vmem;

	if(!(kernel_vmem = new_vmem_obj()))
		goto fail;
	for(i = 0; i < 1024; ++i)
	{
		if(!(vmem = new_vmem_obj()))
		{
			// TODO Free all
			goto fail;
		}
		for(j = 0; j < 1024; ++j)
			vmem[j] = (PAGE_SIZE * (i * 1024 + j))
				| (PAGING_PAGE_WRITE | PAGING_PAGE_PRESENT);
		kernel_vmem[i] = ((uintptr_t) vmem)
			| (PAGING_TABLE_WRITE | PAGING_TABLE_PRESENT);
	}
	paging_enable(kernel_vmem);
	return;

fail:
	// TODO Error
	return;
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
	size_t i;

	if(mem)
	{
		for(i = 0; i < 1024; ++i)
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
	size_t i;
	void *old_page, *new_page;

	if(!(v = new_vmem_obj()))
		return NULL;
	for(i = 0; i < 1024; ++i)
	{
		if(!(from[i] & PAGING_PAGE_PRESENT))
			continue;
		old_page = (void *) (from[i] & PAGING_ADDR_MASK);
		new_page = (mem_dup ? clone_page(old_page) : old_page);
		if(!new_page)
			goto fail;
		v[i] = ((uint32_t) new_page) | (from[i] & PAGING_FLAGS_MASK);
	}
	return v;

fail:
	free_page_table(v, mem_dup);
	return NULL;
}

__attribute__((hot))
vmem_t vmem_clone(vmem_t vmem, const bool mem_dup)
{
	vmem_t v;
	size_t i;
	void *old_table, *new_table;

	if(!vmem || !(v = vmem_init()))
		return NULL;
	for(i = 0; i < 1024; ++i)
	{
		if(!(vmem[i] & PAGING_TABLE_PRESENT))
			continue;
		old_table = (void *) (vmem[i] & PAGING_ADDR_MASK);
		if(!(new_table = clone_page_table(old_table, mem_dup)))
			goto fail;
		v[i] = ((uint32_t) new_table) | (vmem[i] & PAGING_FLAGS_MASK);
	}
	return v;

fail:
	vmem_free(v, false);
	return NULL;
}

__attribute__((hot))
void *vmem_translate(vmem_t vmem, void *ptr)
{
	uintptr_t remain, table, page;
	vmem_t table_obj;

	remain = (uintptr_t) ptr & 0xfff;
	table = ((uintptr_t) ptr >> 12) & 0x3ff;
	page = ((uintptr_t) ptr >> 22) & 0x3ff;
	if(!(vmem[table] & PAGING_TABLE_PRESENT))
		return NULL;
	table_obj = (void *) (vmem[table] & PAGING_ADDR_MASK);
	if(!(table_obj[page] & PAGING_PAGE_PRESENT))
		return NULL;
	return (void *) ((table_obj[page] & PAGING_ADDR_MASK) | remain);
}

__attribute__((hot))
bool vmem_contains(vmem_t vmem, const void *ptr, const size_t size)
{
	// TODO
	(void) vmem;
	(void) ptr;
	(void) size;
	return true;
}

__attribute__((hot))
void *vmem_alloc_pages(vmem_t vmem, const size_t pages)
{
	void *ptr;
	uintptr_t table, page;
	vmem_t table_ptr;

	if(!vmem || !(ptr = buddy_alloc(pages)))
		return NULL;
	table = ((uintptr_t) ptr >> 12) & 0x3ff;
	page = ((uintptr_t) ptr >> 22) & 0x3ff;
	if(!(vmem[table] & PAGING_TABLE_PRESENT))
	{
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
	(void) pages;
	(void) mem_free;
}

__attribute__((hot))
void vmem_free(vmem_t vmem, const bool mem_free)
{
	size_t i;

	if(!vmem)
		return;
	for(i = 0; i < 1024; ++i)
	{
		if(!(vmem[i] & PAGING_TABLE_PRESENT))
			continue;
		free_page_table((void *) (vmem[i] & PAGING_ADDR_MASK), mem_free);
	}
	buddy_free(vmem);
}
