#include <memory/memory.h>
#include <elf/elf.h>
#include <libc/errno.h>

// TODO Use `kernel_vmem` to hide holes in memory?
// TODO Add stack spaces
// TODO Handle shared memory (use free flag into page table entry)
// TODO Pay attention to USER flag

vmem_t kernel_vmem;

__attribute__((hot))
static inline vmem_t new_vmem_obj(void)
{
	return buddy_alloc_zero(0);
}

__attribute__((hot))
vmem_t vmem_init(void)
{
	vmem_t vmem;

	if(!(vmem = new_vmem_obj()))
		return NULL;
	memcpy(vmem, kernel_vmem, PAGE_SIZE);
	return vmem;
}

__attribute__((cold))
static void protect_section(elf_section_header_t *hdr, const char *name)
{
	void *ptr;
	size_t pages;

	(void) name;
	if(hdr->sh_flags & SHF_WRITE || hdr->sh_addralign != PAGE_SIZE)
		return;
	ptr = (void *) hdr->sh_addr;
	pages = UPPER_DIVISION(hdr->sh_size, PAGE_SIZE);
	vmem_identity_range(kernel_vmem, ptr, ptr + (pages * PAGE_SIZE),
		PAGING_PAGE_USER);
}

__attribute__((cold))
static void protect_kernel(void)
{
	iterate_sections(boot_info->elf_sections, boot_info->elf_num,
		boot_info->elf_shndx, boot_info->elf_entsize, protect_section);
}

__attribute__((cold))
void vmem_kernel(void)
{
	if(!(kernel_vmem = new_vmem_obj()))
		goto fail;
	// TODO Fix
	/*vmem_unmap(kernel_vmem, NULL);
	vmem_identity_range(kernel_vmem, (void *) PAGE_SIZE, KERNEL_BEGIN,
		PAGING_PAGE_WRITE);*/
	vmem_identity_range(kernel_vmem, NULL, KERNEL_BEGIN, PAGING_PAGE_WRITE);
	vmem_identity_range(kernel_vmem, KERNEL_BEGIN, heap_begin,
		PAGING_PAGE_WRITE);
	vmem_identity_range(kernel_vmem, heap_begin, memory_end, PAGING_PAGE_WRITE);
	protect_kernel();
	paging_enable(kernel_vmem);
	return;

fail:
	PANIC("Cannot initialize kernel virtual memory!", 0);
}

__attribute__((hot))
void vmem_identity(vmem_t vmem, void *page, const int flags)
{
	vmem_map(vmem, page, page, flags);
}

__attribute__((hot))
void vmem_identity_range(vmem_t vmem, void *from, void *to, int flags)
{
	void *ptr;

	if(!vmem)
		return;
	for(ptr = from; ptr < to; ptr += PAGE_SIZE)
	{
		vmem_identity(vmem, ptr, flags);
		if(errno)
		{
			// TODO Free all
		}
	}
}

__attribute__((hot))
int vmem_is_mapped(vmem_t vmem, void *ptr)
{
	size_t t;
	vmem_t v;

	if(!vmem)
		return 0;
	t = ADDR_TABLE(ptr);
	if(!(vmem[t] & PAGING_TABLE_PRESENT))
		return 0;
	v = (void *) (vmem[t] & PAGING_ADDR_MASK);
	return (v[ADDR_PAGE(ptr)] & PAGING_PAGE_PRESENT);
}

__attribute__((hot))
void vmem_map(vmem_t vmem, void *physaddr, void *virtaddr, const int flags)
{
	size_t t;
	vmem_t v;

	if(!vmem)
		return;
	t = ADDR_TABLE(virtaddr);
	if(!(vmem[t] & PAGING_TABLE_PRESENT))
	{
		if(!(v = new_vmem_obj()))
			return;
		vmem[t] = (uintptr_t) v;
	}
	vmem[t] |= PAGING_TABLE_PRESENT | flags;
	v = (void *) (vmem[t] & PAGING_ADDR_MASK);
	v[ADDR_PAGE(virtaddr)] = (uintptr_t) physaddr | PAGING_PAGE_PRESENT | flags;
}

__attribute__((hot))
void vmem_unmap(vmem_t vmem, void *virtaddr)
{
	size_t t;
	vmem_t v;

	if(!vmem)
		return;
	t = ADDR_TABLE(virtaddr);
	if(!(vmem[t] & PAGING_TABLE_PRESENT))
		return;
	v = (void *) (vmem[t] & PAGING_ADDR_MASK);
	v[ADDR_PAGE(virtaddr)] = 0;
	// TODO Free page table if empty
}

__attribute__((hot))
static void free_page_table(vmem_t table, const int mem)
{
	size_t i;

	if(!table)
		return;
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
static vmem_t clone_page_table(vmem_t from, const int mem_dup)
{
	vmem_t v;
	size_t i;
	void *old_page, *new_page;

	if(!from || !(v = new_vmem_obj()))
		return NULL;
	for(i = 0; i < 1024; ++i)
	{
		if(!(from[i] & PAGING_PAGE_PRESENT))
			continue;
		if((from[i] & PAGING_TABLE_USER))
		{
			old_page = (void *) (from[i] & PAGING_ADDR_MASK);
			if(!(new_page = (mem_dup ? clone_page(old_page) : old_page)))
				goto fail;
			v[i] = ((uint32_t) new_page) | (from[i] & PAGING_FLAGS_MASK);
		}
		else
			v[i] = from[i];
	}
	return v;

fail:
	free_page_table(v, mem_dup);
	return NULL;
}

__attribute__((hot))
vmem_t vmem_clone(vmem_t vmem, const int mem_dup)
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
		if((vmem[i] & PAGING_TABLE_USER))
		{
			old_table = (void *) (vmem[i] & PAGING_ADDR_MASK);
			if(!(new_table = clone_page_table(old_table, mem_dup)))
				goto fail;
			v[i] = ((uint32_t) new_table) | (vmem[i] & PAGING_FLAGS_MASK);
		}
		else
			v[i] = vmem[i];
	}
	return v;

fail:
	vmem_free(v, 0);
	return NULL;
}

__attribute__((hot))
void *vmem_translate(vmem_t vmem, void *ptr)
{
	uintptr_t table, page, remain;
	vmem_t table_obj;

	if(!vmem)
		return NULL;
	table = ADDR_TABLE(ptr);
	page = ADDR_PAGE(ptr);
	remain = ADDR_REMAIN(ptr);
	if(!(vmem[table] & PAGING_TABLE_PRESENT))
		return NULL;
	table_obj = (void *) (vmem[table] & PAGING_ADDR_MASK);
	if(!(table_obj[page] & PAGING_PAGE_PRESENT))
		return NULL;
	return (void *) ((table_obj[page] & PAGING_ADDR_MASK) | remain);
}

__attribute__((hot))
int vmem_contains(vmem_t vmem, const void *ptr, const size_t size)
{
	void *i;

	if(!vmem)
		return 0;
	i = ALIGN_DOWN(ptr, PAGE_SIZE);
	while(i < ptr + size)
	{
		if(!vmem_is_mapped(vmem, i))
			return 0;
		i += PAGE_SIZE;
	}
	return 1;
}

__attribute__((hot))
void *vmem_alloc_pages(vmem_t vmem, const size_t pages)
{
	void *ptr;

	if(!vmem || !(ptr = pages_alloc_zero(pages)))
		return NULL;
	// TODO Map at specific places for stacks
	vmem_identity_range(vmem, ptr, ptr + (pages * PAGE_SIZE),
		PAGING_PAGE_USER | PAGING_PAGE_WRITE);
	if(errno)
	{
		pages_free(ptr, pages);
		return NULL;
	}
	return ptr;
}

// TODO Use vmem_unmap to 
__attribute__((hot))
void vmem_free_pages(vmem_t vmem, const size_t pages, const int mem_free)
{
	if(!vmem)
		return;
	// TODO
	(void) pages;
	(void) mem_free;
}

__attribute__((hot))
void vmem_free(vmem_t vmem, const int mem_free)
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
