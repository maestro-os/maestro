#include <memory/vmem/vmem.h>
#include <memory/buddy/buddy.h>
#include <elf/elf.h>
#include <debug/debug.h>

#include <libc/errno.h>

/*
 * Tells whether the mapping can use PSE or not for the given `ptr`.
 */
#define CAN_USE_PSE(ptr, remaining_pages)\
	(IS_ALIGNED((ptr), 0x400000) && (remaining_pages) >= 1024)

/*
 * This file handles x86 memory permissions handling.
 * x86 uses a tree-like structure to handle permissions. This structure is made
 * of several objects:
 * - Page directory: A 1024 entries array containing page tables
 * - Page table: A 1024 entries array describing permissions on each page
 *
 * Both objects are 4096 bytes large.
 *
 * PSE (Page Size Extention) allows to map large blocks of 1024 pages without
 * using a page table.
 *
 * Paging objects are read-only, meaning that the kernel must unlock writing to
 * read only pages to modify them.
 */

/*
 * The kernel's memory context.
 */
vmem_t kernel_vmem = NULL;

/*
 * Creates a paging object.
 */
ATTR_HOT
static inline vmem_t vmem_obj_new(void)
{
	// TODO Map without writing permission?
	return buddy_alloc_zero(0, BUDDY_FLAG_ZONE_KERNEL);
}

/*
 * Initializes a new page directory. By default, the page directory is a copy
 * of the kernel's page directory.
 */
ATTR_HOT
vmem_t vmem_init(void)
{
	vmem_t vmem;

	if(!(vmem = vmem_obj_new()))
		return NULL;
	// TODO If Meltdown mitigation is enabled, only allow read access to a stub for interrupts
	vmem_map_range(vmem, NULL, PROCESS_END, 262144, PAGING_PAGE_WRITE);
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
	vmem_map_range(kernel_vmem, (void *) (PROCESS_END + (uintptr_t) ptr), ptr,
		pages, PAGING_PAGE_USER);
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
 * Creates the kernel's page directory.
 */
ATTR_COLD
void vmem_kernel(void)
{
	if(!(kernel_vmem = vmem_init()))
		goto fail;
	protect_kernel();
	paging_enable(kernel_vmem);
	return;

fail:
	PANIC("Cannot initialize kernel virtual memory!", 0);
}

/*
 * Resolves the paging entry for the given pointer. If no entry is found, `NULL`
 * is returned. The entry must be marked as present to be found.
 * If Page Size Extention (PSE) is used, an entry of the page directory might
 * be returned.
 */
ATTR_HOT
uint32_t *vmem_resolve(vmem_t vmem, const void *ptr)
{
	uintptr_t table, page;
	vmem_t table_obj;

	debug_assert(sanity_check(vmem), "vmem: invalid arguments");
	table = ADDR_TABLE(ptr);
	if(!(vmem[table] & PAGING_TABLE_PRESENT))
		return NULL;
	if(vmem[table] & PAGING_TABLE_PAGE_SIZE)
		return &vmem[table];
	table_obj = (void *) (vmem[table] & PAGING_ADDR_MASK);
	page = ADDR_PAGE(ptr);
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
 * Checks if the portion of memory beginning at `ptr` with size `size` is
 * mapped.
 */
ATTR_HOT
int vmem_contains(vmem_t vmem, const void *ptr, const size_t size)
{
	void *i;

	debug_assert(sanity_check(vmem), "vmem: invalid arguments");
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

	debug_assert(sanity_check(vmem), "vmem: invalid arguments");
	if(!sanity_check(entry = vmem_resolve(vmem, ptr)))
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

	debug_assert(sanity_check(vmem), "vmem: invalid arguments");
	if(!sanity_check(entry = vmem_resolve(vmem, ptr)))
		return 0;
	return *entry & PAGING_FLAGS_MASK;
}

/*
 * Checks if the given `entry` is present, if it is present and not a large
 * block, then an empty table is created. If the entry contains a large block, a
 * table is created and the entries are filled to match the previous mapping.
 */
static void vmem_table_check(uint32_t *entry, uint32_t flags)
{
	vmem_t v;
	void *ptr;
	size_t i;

	debug_assert(sanity_check(entry), "vmem: invalid argument");
	if(!(*entry & PAGING_TABLE_PRESENT))
	{
		if(!(v = vmem_obj_new()))
			return;
	}
	else if(*entry & PAGING_TABLE_PAGE_SIZE)
	{
		if(!(v = vmem_obj_new()))
			return;
		ptr = (void *) (*entry & PAGING_ADDR_MASK);
		for(i = 0; i < 1024; ++i)
			v[i] = (uint32_t) (ptr + (i * PAGE_SIZE))
				| flags | PAGING_PAGE_PRESENT;
	}
	*entry = (uint32_t) v | flags | PAGING_TABLE_PRESENT;
}

/*
 * If the entry contains a table, frees it.
 */
static void vmem_table_pse_clear(uint32_t *entry)
{
	debug_assert(sanity_check(entry), "vmem: invalid argument");
	if(!(*entry & PAGING_TABLE_PRESENT) || (*entry & PAGING_TABLE_PAGE_SIZE))
		return;
	buddy_free((void *) (*entry & PAGING_ADDR_MASK), 0);
	*entry = 0;
}

/*
 * Maps the given physical address to the given virtual address with the given
 * flags.
 */
ATTR_HOT
void vmem_map(vmem_t vmem, const void *physaddr, const void *virtaddr,
	int flags)
{
	size_t t;
	vmem_t v;
	uint32_t lock;

	errno = 0;
	debug_assert(sanity_check(vmem), "vmem: invalid arguments");
	physaddr = DOWN_ALIGN(physaddr, PAGE_SIZE);
	virtaddr = DOWN_ALIGN(virtaddr, PAGE_SIZE);
	t = ADDR_TABLE(virtaddr);
	vmem_table_check(&vmem[t], flags);
	v = (void *) (vmem[t] & PAGING_ADDR_MASK);
	lock = cr0_get() & 0x10000;
	cr0_clear(lock);
	v[ADDR_PAGE(virtaddr)] = (uintptr_t) physaddr | PAGING_PAGE_PRESENT | flags;
	cr0_set(lock);
	vmem_flush(vmem);
}

/*
 * Maps the given physical address to the given virtual address with the given
 * flags using blocks of 1024 pages (PSE).
 */
void vmem_map_pse(vmem_t vmem, const void *physaddr, const void *virtaddr,
	int flags)
{
	size_t t;

	debug_assert(sanity_check(vmem), "vmem: invalid arguments");
	physaddr = DOWN_ALIGN(physaddr, PAGE_SIZE);
	virtaddr = DOWN_ALIGN(virtaddr, PAGE_SIZE);
	t = ADDR_TABLE(virtaddr);
	vmem_table_pse_clear(&vmem[t]);
	vmem[t] = (uintptr_t) physaddr | flags | PAGING_TABLE_PAGE_SIZE
		| PAGING_TABLE_PRESENT;
}

/*
 * Maps the specified range of physical memory to the specified range of virtual
 * memory.
 */
void vmem_map_range(vmem_t vmem, const void *physaddr, const void *virtaddr,
	const size_t pages, const int flags)
{
	size_t i = 0;
	const void *v, *p;

	debug_assert(sanity_check(vmem)
		&& (size_t) physaddr / PAGE_SIZE + pages <= 1048576
		&& (size_t) virtaddr / PAGE_SIZE + pages <= 1048576,
		"vmem: invalid arguments");
	while(i < pages)
	{
		v = virtaddr + i * PAGE_SIZE;
		p = physaddr + i * PAGE_SIZE;
		if(CAN_USE_PSE(v, pages - i))
		{
			vmem_map_pse(vmem, p, v, flags);
			i += 1024;
		}
		else
		{
			vmem_map(vmem, p, v, flags);
			++i;
		}
		if(errno)
		{
			vmem_unmap_range(vmem, virtaddr, pages);
			return;
		}
	}
}

/*
 * Identity mapping of the given page (maps the given page to the same virtual
 * address as its physical address).
 */
ATTR_HOT
void vmem_identity(vmem_t vmem, const void *page, const int flags)
{
	vmem_map(vmem, page, page, flags);
}

/*
 * Identity mapping of the given pages (maps the given page to the same virtual
 * address as its physical address), using blocks of 1024 pages (PSE).
 */
ATTR_HOT
void vmem_identity_pse(vmem_t vmem, const void *page, int flags)
{
	vmem_map_pse(vmem, page, page, flags);
}

/*
 * Identity maps a range of pages.
 */
ATTR_HOT
void vmem_identity_range(vmem_t vmem, const void *from, const size_t pages,
	int flags)
{
	size_t i = 0;
	const void *ptr;

	debug_assert(sanity_check(vmem)
		&& (size_t) from / PAGE_SIZE + pages < 1048576,
		"vmem: invalid arguments");
	while(i < pages)
	{
		ptr = from + i * PAGE_SIZE;
		if(CAN_USE_PSE(ptr, pages - i))
		{
			vmem_identity_pse(vmem, ptr, flags);
			i += 1024;
		}
		else
		{
			vmem_identity(vmem, ptr, flags);
			++i;
		}
		if(errno)
		{
			vmem_unmap_range(vmem, from, pages);
			return;
		}
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

	debug_assert(sanity_check(vmem), "vmem: invalid arguments");
	t = ADDR_TABLE(virtaddr);
	if(!(vmem[t] & PAGING_TABLE_PRESENT))
		return;
	vmem_table_check(&vmem[t], vmem[t] & PAGING_FLAGS_MASK);
	v = (void *) (vmem[t] & PAGING_ADDR_MASK);
	lock = cr0_get() & 0x10000;
	cr0_clear(lock);
	v[ADDR_PAGE(virtaddr)] = 0;
	cr0_set(lock);
	// TODO If page table is empty, free it
	vmem_flush(vmem);
}

/*
 * Unmaps the given virtual memory range.
 */
void vmem_unmap_range(vmem_t vmem, const void *virtaddr, const size_t pages)
{
	size_t i;

	debug_assert(sanity_check(vmem), "vmem: invalid arguments");
	// TODO Optimize for PSE
	for(i = 0; i < pages; ++i)
		vmem_unmap(vmem, virtaddr + i * PAGE_SIZE);
}

/*
 * Clones the given page table.
 */
ATTR_HOT
static vmem_t clone_page_table(vmem_t from)
{
	vmem_t v;

	debug_assert(sanity_check(from), "vmem: invalid argument");
	if(!(v = vmem_obj_new()))
		return NULL;
	memcpy(v, from, PAGE_SIZE);
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

	debug_assert(sanity_check(vmem), "vmem: invalid argument");
	if(!(v = vmem_init()))
		return NULL;
	lock = cr0_get() & 0x10000;
	cr0_clear(lock);
	errno = 0;
	for(i = 0; i < 1024; ++i)
	{
		if(!(vmem[i] & PAGING_TABLE_PRESENT))
			continue;
		if(!(vmem[i] & PAGING_TABLE_PAGE_SIZE))
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
	debug_assert(sanity_check(vmem), "vmem: invalid argument");
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

	debug_assert(sanity_check(vmem), "vmem: invalid argument");
	for(i = 0; i < 1024; ++i)
	{
		if(!(vmem[i] & PAGING_TABLE_PRESENT)
			|| (vmem[i] & PAGING_TABLE_PAGE_SIZE))
			continue;
		buddy_free((void *) (vmem[i] & PAGING_ADDR_MASK), 0);
	}
	buddy_free(vmem, 0);
}
