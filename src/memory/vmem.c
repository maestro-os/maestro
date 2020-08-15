#include <memory/memory.h>
#include <elf/elf.h>
#include <debug/debug.h>

#include <libc/errno.h>

/*
 * This file handles x86 memory permissions handling.
 * x86 uses a tree-like structure to handle permissions. This structure is made
 * of several elements:
 * - Page directory: A 1024 entries array containing page tables
 * - Page table: A 1024 entries array describing permissions on each page
 *
 * Both objects are 4096 bytes large.
 *
 * After modifying a page directory, function `vmem_flush` should be called.
 */

/*
 * The kernel's memory context.
 */
vmem_t kernel_vmem;

/*
 * Creates a paging object.
 */
ATTR_HOT
static inline vmem_t new_vmem_obj(void)
{
	return buddy_alloc_zero(0);
}

/*
 * Initializes a new page directory. By default, the page directory is a copy
 * of the kernel's page directory.
 */
ATTR_HOT
vmem_t vmem_init(void)
{
	vmem_t vmem;

	if(!(vmem = new_vmem_obj()))
		return NULL;
	// TODO Only allow read access to a stub for interrupts
	vmem_identity_range(vmem, KERNEL_PHYS_BEGIN,
		(mem_info.heap_begin - KERNEL_PHYS_BEGIN) / PAGE_SIZE,
		PAGING_PAGE_USER);
	return vmem;
}

/*
 * Protects write-protected section specified by the ELF section header given
 * in argument.
 */
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
	vmem_identity_range(kernel_vmem, ptr, pages, PAGING_PAGE_USER);
}

/*
 * Protects the kernel code.
 */
ATTR_COLD
static void protect_kernel(void)
{
	iterate_sections(boot_info.elf_sections, boot_info.elf_num,
		boot_info.elf_shndx, boot_info.elf_entsize, protect_section);
}

/*
 * Protects the tables inside of the given directory.
 */
static void protect_tables(vmem_t vmem)
{
	size_t i;
	void *table_ptr;
	uint32_t *table_entry;

	for(i = 0; i < 1024; ++i)
	{
		if(!(vmem[i] & PAGING_TABLE_PRESENT))
			continue;
		table_ptr = (void *) (vmem[i] & PAGING_ADDR_MASK);
		if(!(table_entry = vmem_resolve(vmem, table_ptr)))
			continue;
		*table_entry &= ~PAGING_PAGE_WRITE;
	}
}

/*
 * Creates the kernel's page directory.
 */
ATTR_COLD
void vmem_kernel(void)
{
	if(!(kernel_vmem = new_vmem_obj()))
		goto fail;
	vmem_unmap(kernel_vmem, NULL);
	vmem_identity_range(kernel_vmem, (void *) PAGE_SIZE,
		(mem_info.memory_end - (void *) PAGE_SIZE) / PAGE_SIZE,
			PAGING_PAGE_WRITE);
	protect_kernel();
	protect_tables(kernel_vmem);
	paging_enable(kernel_vmem);
	return;

fail:
	PANIC("Cannot initialize kernel virtual memory!", 0);
}

/*
 * Resolves the paging entry for the given pointer. If no entry is found, `NULL`
 * is returned. The entry must be marked as present to be found.
 */
ATTR_HOT
uint32_t *vmem_resolve(vmem_t vmem, const void *ptr)
{
	uintptr_t table, page;
	vmem_t table_obj;

	if(!sanity_check(vmem))
		return NULL;
	table = ADDR_TABLE(ptr);
	page = ADDR_PAGE(ptr);
	if(!(vmem[table] & PAGING_TABLE_PRESENT))
		return NULL;
	table_obj = (void *) (vmem[table] & PAGING_ADDR_MASK);
	if(!(table_obj[page] & PAGING_PAGE_PRESENT))
		return NULL;
	return &table_obj[page];
}

/*
 * Checks if the given pointer is mapped.
 */
ATTR_HOT
int vmem_is_mapped(vmem_t vmem, const void *ptr)
{
	return (vmem_resolve(vmem, ptr) != NULL);
}

/*
 * Maps the given physical address to the given virtual address with the given
 * flags.
 */
ATTR_HOT
void vmem_map(vmem_t vmem, const void *physaddr, const void *virtaddr,
	const int flags)
{
	size_t t;
	vmem_t v;
	uint32_t lock;

	errno = 0;
	if(!sanity_check(vmem))
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
	lock = cr0_get() & 0x10000;
	cr0_clear(lock);
	v[ADDR_PAGE(virtaddr)] = (uintptr_t) physaddr | PAGING_PAGE_PRESENT | flags;
	cr0_set(lock);
	tlb_reload();
}

/*
 * Maps the specified range of physical memory to the specified range of virtual
 * memory.
 */
void vmem_map_range(vmem_t vmem, const void *physaddr, const void *virtaddr,
	const size_t pages, const int flags)
{
	size_t i = 0;

	if(!sanity_check(vmem))
		return;
	while(i < pages)
	{
		vmem_map(vmem, physaddr + i * PAGE_SIZE,
			virtaddr + i * PAGE_SIZE, flags);
		if(errno)
		{
			vmem_unmap_range(vmem, virtaddr, pages);
			return;
		}
		++i;
	}
}

/*
 * Identity mapping of the given page. (maps the given page to the same virtual
 * address as its physical address)
 */
ATTR_HOT
void vmem_identity(vmem_t vmem, const void *page, const int flags)
{
	vmem_map(vmem, page, page, flags);
}

/*
 * Identity maps a range of pages.
 */
ATTR_HOT
void vmem_identity_range(vmem_t vmem, const void *from, const size_t pages,
	int flags)
{
	size_t i = 0;

	if(!vmem)
		return;
	while(i < pages)
	{
		vmem_identity(vmem, from + i * PAGE_SIZE, flags);
		if(errno)
		{
			vmem_unmap_range(vmem, from, pages);
			return;
		}
		++i;
	}
}

/*
 * Unmaps the given virtual address.
 */
ATTR_HOT
void vmem_unmap(vmem_t vmem, const void *virtaddr)
{
	size_t t;
	vmem_t v;
	uint32_t lock;

	if(!sanity_check(vmem))
		return;
	t = ADDR_TABLE(virtaddr);
	if(!(vmem[t] & PAGING_TABLE_PRESENT))
		return;
	v = (void *) (vmem[t] & PAGING_ADDR_MASK);
	lock = cr0_get() & 0x10000;
	cr0_clear(lock);
	v[ADDR_PAGE(virtaddr)] = 0;
	cr0_set(lock);
	// TODO If page table is empty, free it
	tlb_reload();
}

// TODO Optimize
/*
 * Unmaps the given virtual memory range.
 */
void vmem_unmap_range(vmem_t vmem, const void *virtaddr, const size_t pages)
{
	size_t i = 0;

	if(!sanity_check(vmem))
		return;
	while(i < pages)
	{
		vmem_unmap(vmem, virtaddr + i * PAGE_SIZE);
		++i;
	}

}

/*
 * Checks if the portion of memory beginning at `ptr` with size `size` is
 * mapped.
 */
ATTR_HOT
int vmem_contains(vmem_t vmem, const void *ptr, const size_t size)
{
	void *i;

	if(!sanity_check(vmem))
		return 0;
	i = DOWN_ALIGN(ptr, PAGE_SIZE);
	while(i < ptr + size)
	{
		if(!vmem_is_mapped(vmem, i))
			return 0;
		i += PAGE_SIZE;
	}
	return 1;
}

/*
 * Translates the given virtual address to the corresponding physical address.
 * If the address is not mapped, `NULL` is returned.
 */
ATTR_HOT
void *vmem_translate(vmem_t vmem, const void *ptr)
{
	uint32_t *entry;

	if(!sanity_check(vmem) || !sanity_check(entry = vmem_resolve(vmem, ptr)))
		return NULL;
	return (void *) ((*entry & PAGING_ADDR_MASK) | ADDR_REMAIN(ptr));
}

/*
 * Resolves the entry for the given virtual address and returns its flags.
 */
ATTR_HOT
uint32_t vmem_get_entry(vmem_t vmem, const void *ptr)
{
	uint32_t *entry;

	if(!sanity_check(vmem) || !sanity_check(entry = vmem_resolve(vmem, ptr)))
		return 0;
	return *entry & PAGING_FLAGS_MASK;
}

/*
 * Clones the given page table.
 */
ATTR_HOT
static vmem_t clone_page_table(vmem_t from)
{
	vmem_t v;

	if(!(v = new_vmem_obj()))
		return NULL;
	memcpy(v, sanity_check(from), PAGE_SIZE);
	return v;
}

/*
 * Clones the given page directory and tables in it.
 */
ATTR_HOT
vmem_t vmem_clone(vmem_t vmem)
{
	vmem_t v;
	uint32_t lock;
	size_t i;
	void *old_table, *new_table;

	if(!sanity_check(vmem) || !(v = vmem_init()))
		return NULL;
	lock = cr0_get() & 0x10000;
	cr0_clear(lock);
	errno = 0;
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
	cr0_set(lock);
	return v;

fail:
	vmem_destroy(v);
	cr0_set(lock);
	return NULL;
}

/*
 * Flushes modifications to the given page directory.
 */
void vmem_flush(vmem_t vmem)
{
	if(!sanity_check(vmem))
		return;
	if(vmem == cr3_get())
		tlb_reload();
}

/*
 * Destroyes the given page directory.
 */
ATTR_HOT
void vmem_destroy(vmem_t vmem)
{
	size_t i;

	if(!sanity_check(vmem))
		return;
	for(i = 0; i < 1024; ++i)
	{
		if(!(vmem[i] & PAGING_TABLE_PRESENT))
			continue;
		buddy_free((void *) (vmem[i] & PAGING_ADDR_MASK), 0);
	}
	buddy_free(vmem, 0);
}
