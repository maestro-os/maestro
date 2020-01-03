#include <memory/memory.h>
#include <elf/elf.h>
#include <libc/errno.h>

// TODO Use `kernel_vmem` to hide holes in memory?

vmem_t kernel_vmem;

ATTR_HOT
static inline vmem_t new_vmem_obj(void)
{
	return buddy_alloc_zero(0);
}

ATTR_HOT
vmem_t vmem_init(void)
{
	vmem_t vmem;

	if(!(vmem = new_vmem_obj()))
		return NULL;
	// TODO Only allow read access to kernel (or just a stub for interrupts)
	memcpy(vmem, kernel_vmem, PAGE_SIZE); // TODO rm
	return vmem;
}

ATTR_COLD
static void protect_section(elf_section_header_t *hdr, const char *name)
{
	void *ptr;
	size_t pages;

	(void) name;
	if(hdr->sh_flags & SHF_WRITE || hdr->sh_addralign != PAGE_SIZE)
		return;
	ptr = (void *) hdr->sh_addr;
	pages = CEIL_DIVISION(hdr->sh_size, PAGE_SIZE);
	vmem_identity_range(kernel_vmem, ptr, ptr + (pages * PAGE_SIZE),
		PAGING_PAGE_USER);
}

ATTR_COLD
static void protect_kernel(void)
{
	iterate_sections(boot_info->elf_sections, boot_info->elf_num,
		boot_info->elf_shndx, boot_info->elf_entsize, protect_section);
}

ATTR_COLD
void vmem_kernel(void)
{
	if(!(kernel_vmem = new_vmem_obj()))
		goto fail;
	// TODO Fix
	vmem_unmap(kernel_vmem, NULL);
	vmem_identity_range(kernel_vmem, (void *) PAGE_SIZE, KERNEL_BEGIN,
		PAGING_PAGE_WRITE);
	vmem_identity_range(kernel_vmem, NULL, KERNEL_BEGIN, PAGING_PAGE_WRITE);
	vmem_identity_range(kernel_vmem, KERNEL_BEGIN, mem_info.heap_begin,
		PAGING_PAGE_WRITE);
	// TODO Do not grant access to processes' pages (accessible only during syscalls?)
	vmem_identity_range(kernel_vmem, mem_info.heap_begin, mem_info.memory_end,
		PAGING_PAGE_WRITE); // TODO rm?
	protect_kernel();
	paging_enable(kernel_vmem);
	return;

fail:
	PANIC("Cannot initialize kernel virtual memory!", 0);
}

ATTR_HOT
void vmem_identity(vmem_t vmem, void *page, const int flags)
{
	vmem_map(vmem, page, page, flags);
}

ATTR_HOT
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
			// TODO Unmap range
		}
	}
}

ATTR_HOT
uint32_t *vmem_resolve(vmem_t vmem, void *ptr)
{
	uintptr_t table, page;
	vmem_t table_obj;

	table = ADDR_TABLE(ptr);
	page = ADDR_PAGE(ptr);
	if(!(vmem[table] & PAGING_TABLE_PRESENT))
		return NULL;
	table_obj = (void *) (vmem[table] & PAGING_ADDR_MASK);
	if(!(table_obj[page] & PAGING_PAGE_PRESENT))
		return NULL;
	return table_obj + page;
}

ATTR_HOT
int vmem_is_mapped(vmem_t vmem, void *ptr)
{
	return (vmem_resolve(vmem, ptr) != NULL);
}

// TODO Reload tlb after mapping?
ATTR_HOT
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

ATTR_HOT
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
	// TODO If page table is empty, free it
}

ATTR_HOT
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

ATTR_HOT
void *vmem_translate(vmem_t vmem, void *ptr)
{
	uint32_t *entry;

	if(!vmem || !(entry = vmem_resolve(vmem, ptr)))
		return NULL;
	return (void *) ((*entry & PAGING_ADDR_MASK) | ADDR_REMAIN(ptr));
}

ATTR_HOT
uint32_t vmem_get_entry(vmem_t vmem, void *ptr)
{
	uint32_t *entry;

	if(!vmem || !(entry = vmem_resolve(vmem, ptr)))
		return 0;
	return *entry & PAGING_FLAGS_MASK;

}

ATTR_HOT
static vmem_t clone_page_table(vmem_t from)
{
	vmem_t v;

	if(!from || !(v = new_vmem_obj()))
		return NULL;
	memcpy(v, from, PAGE_SIZE);
	return v;
}

ATTR_HOT
vmem_t vmem_clone(vmem_t vmem)
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
			if(!(new_table = clone_page_table(old_table)))
				goto fail;
			v[i] = ((uint32_t) new_table) | (vmem[i] & PAGING_FLAGS_MASK);
		}
		else
			v[i] = vmem[i];
	}
	return v;

fail:
	vmem_destroy(v);
	return NULL;
}

ATTR_HOT
void vmem_destroy(vmem_t vmem)
{
	size_t i;

	if(!vmem)
		return;
	for(i = 0; i < 1024; ++i)
	{
		if(!(vmem[i] & PAGING_TABLE_PRESENT))
			continue;
		buddy_free((void *) (vmem[i] & PAGING_ADDR_MASK));
	}
	buddy_free(vmem);
}
