#ifndef MEMORY_H
# define MEMORY_H

# include <multiboot.h>
# include <memory/buddy/buddy.h>
# include <memory/kmalloc/kmalloc.h>
# include <memory/slab/slab.h>
# include <util/util.h>

/*
 * The size of a page in bytes.
 */
# define PAGE_SIZE	((size_t) 0x1000)

/*
 * The virtual pointer to the beginning of the kernel.
 */
# define KERNEL_VIRT_BEGIN	((void *) &kernel_begin)
/*
 * The virtual pointer to the end of the kernel.
 */
# define KERNEL_VIRT_END	((void *) &kernel_end)
/*
 * The kernel's image size in bytes.
 */
# define KERNEL_SIZE		((size_t) (KERNEL_VIRT_END - KERNEL_VIRT_BEGIN))
/*
 * The physical pointer to the beginning of the kernel.
 */
# define KERNEL_PHYS_BEGIN	((void *) 0x100000)
/*
 * The physical pointer to the end of the kernel.
 */
# define KERNEL_PHYS_END	(KERNEL_PHYS_BEGIN + KERNEL_SIZE)

/*
 * The pointer to the end of the virtual memory reserved to the process.
 */
# define PROCESS_END		((void *) 0xc0000000)

/*
 * x86 paging flag. If set, pages are 4 MB long.
 */
# define PAGING_TABLE_PAGE_SIZE		0b10000000
/*
 * x86 paging flag. Set if the page has been read or wrote.
 */
# define PAGING_TABLE_ACCESSED		0b00100000
/*
 * x86 paging flag. If set, page will not be cached.
 */
# define PAGING_TABLE_CACHE_DISABLE	0b00010000
/*
 * x86 paging flag. If set, write-through caching is enabled.
 * If not, then write-back is enabled instead.
 */
# define PAGING_TABLE_WRITE_THROUGH	0b00001000
/*
 * x86 paging flag. If set, the page can be accessed by userspace operations.
 */
# define PAGING_TABLE_USER			0b00000100
/*
 * x86 paging flag. If set, the page can be wrote.
 */
# define PAGING_TABLE_WRITE			0b00000010
/*
 * x86 paging flag. If set, the page is present.
 */
# define PAGING_TABLE_PRESENT		0b00000001

# define PAGING_PAGE_GLOBAL			0b100000000
# define PAGING_PAGE_DIRTY			0b001000000
# define PAGING_PAGE_ACCESSED		0b000100000
# define PAGING_PAGE_CACHE_DISABLE	0b000010000
# define PAGING_PAGE_WRITE_THROUGH	0b000001000
# define PAGING_PAGE_USER			0b000000100
# define PAGING_PAGE_WRITE			0b000000010
# define PAGING_PAGE_PRESENT		0b000000001

/*
 * Flags mask in a page directory entry.
 */
# define PAGING_FLAGS_MASK	0xfff
/*
 * Address mask in a page directory entry. The address doesn't need every bytes
 * since it must be page-aligned.
 */
# define PAGING_ADDR_MASK	~((uint32_t) PAGING_FLAGS_MASK)

/*
 * Converts the page number to a pointer to the beginning of the pages.
 */
# define PAGETOPTR(page)	((void *) (page) * PAGE_SIZE)
/*
 * Converts a pointer to the page index containing it.
 */
# define PTRTOPAGE(ptr)		((uintptr_t) (ptr) / PAGE_SIZE)

/*
 * Gives the table index for the given address.
 */
# define ADDR_TABLE(addr)	(((uintptr_t) (addr) >> 22) & 0x3ff)
/*
 * Gives the page index for the given address.
 */
# define ADDR_PAGE(addr)	(((uintptr_t) (addr) >> 12) & 0x3ff)
/*
 * Gives the offset of the pointer in its page.
 */
# define ADDR_REMAIN(addr)	((uintptr_t) (addr) & 0xfff)

/*
 * x86 page fault flag. If set, the page was present.
 */
# define PAGE_FAULT_PRESENT		0b00001
/*
 * x86 page fault flag. If set, the error was caused bt a write operation, else
 * the error was caused by a read operation.
 */
# define PAGE_FAULT_WRITE		0b00010
/*
 * x86 page fault flag. If set, the page fault was caused by a userspace
 * operation.
 */
# define PAGE_FAULT_USER		0b00100
/*
 * x86 page fault flag. If set, one or more page directory entries contain
 * reserved bits which are set.
 */
# define PAGE_FAULT_RESERVED	0b01000
/*
 * x86 page fault flag. If set, the page fault was caused by an instruction
 * fetch.
 */
# define PAGE_FAULT_INSTRUCTION	0b10000

/*
 * Structure storing informations relative to the main memory.
 */
typedef struct
{
	/* Size of the Multiboot2 memory map */
	size_t memory_maps_size;
	/* Size of an entry in the Multiboot2 memory map */
	size_t memory_maps_entry_size;
	/* Pointer to the Multiboot2 memory map */
	void *memory_maps;

	/* Pointer to the end of the physical memory */
	void *memory_end;
	/* Pointer to the beginning of physical allocatable memory */
	void *phys_alloc_begin;
	/* Pointer to the end of physical allocatable memory */
	void *phys_alloc_end;
	/* The amount total of allocatable memory */
	size_t available_memory;
} memory_info_t;

/*
 * Structure containing the memory usage.
 */
typedef struct
{
	/* The amount of reserved memory that the kernel cannot use */
	size_t reserved;
	/* The amount of bad memory */
	size_t bad_ram;
	/* The amount of memory used by the kernel itself */
	size_t system;
	/* The amount of allocated memory (kernel allocations included) */
	size_t allocated;
	/* The amount of remaining free memory */
	size_t free;
} mem_usage_t;

/*
 * The object used in x86 memory permissions handling.
 */
typedef uint32_t *vmem_t;

extern int kernel_begin;
extern int kernel_end;

extern memory_info_t mem_info;
extern vmem_t kernel_vmem;

extern int check_a20(void);
void enable_a20(void);

void memmap_init(void *multiboot_ptr);
void memmap_print(void);
const char *memmap_type(uint32_t type);

void print_mem_amount(size_t amount);
void *clone_page(void *ptr);

void get_memory_usage(mem_usage_t *usage);
# ifdef KERNEL_DEBUG
void print_mem_usage(void);
# endif

vmem_t vmem_init(void);
void vmem_kernel(void);
uint32_t *vmem_resolve(vmem_t vmem, const void *ptr);
int vmem_is_mapped(vmem_t vmem, const void *ptr);
void vmem_map(vmem_t vmem, const void *physaddr, const void *virtaddr,
	int flags);
void vmem_map_range(vmem_t vmem, const void *physaddr, const void *virtaddr,
	size_t pages, int flags);
void vmem_identity(vmem_t vmem, const void *page, int flags);
void vmem_identity_range(vmem_t vmem, const void *from, size_t pages,
	int flags);
void vmem_unmap(vmem_t vmem, const void *virtaddr);
void vmem_unmap_range(vmem_t vmem, const void *virtaddr, size_t pages);
int vmem_contains(vmem_t vmem, const void *ptr, size_t size);
void *vmem_translate(vmem_t vmem, const void *ptr);
uint32_t vmem_get_entry(vmem_t vmem, const void *ptr);
vmem_t vmem_clone(vmem_t vmem);
void vmem_flush(vmem_t vmem);
void vmem_destroy(vmem_t vmem);

extern uint32_t cr0_get(void);
extern void cr0_set(uint32_t flags);
extern void cr0_clear(uint32_t flags);
extern void *cr2_get(void);
extern void *cr3_get(void);

extern void paging_enable(vmem_t vmem);
extern void paging_disable(void);
extern void tlb_reload(void);

#endif
