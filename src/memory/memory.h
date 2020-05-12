#ifndef MEMORY_H
# define MEMORY_H

# include <multiboot.h>
# include <memory/buddy/buddy.h>
# include <memory/pages/pages.h>
# include <memory/kmalloc/kmalloc.h>
# include <memory/slab/slab.h>
# include <util/util.h>

/*
 * The size of a page in bytes.
 */
# define PAGE_SIZE		((size_t) 0x1000)
/*
 * The pointer to the beginning of the kernel.
 */
# define KERNEL_BEGIN	((void *) 0x100000)

/*
 * Memory region flag allowing write permission on the region.
 */
# define MEM_REGION_FLAG_WRITE		0b000001
/*
 * Memory region flag allowing execution permission on the region.
 */
# define MEM_REGION_FLAG_EXEC		0b000010
/*
 * Memory region flag telling that the region is shared with other memory
 * spaces.
 */
# define MEM_REGION_FLAG_SHARED		0b000100
/*
 * Memory region flag telling that the region is a stack.
 */
# define MEM_REGION_FLAG_STACK		0b001000
/*
 * Memory region flag telling that the region is a userspace region.
 */
# define MEM_REGION_FLAG_USER		0b010000
/*
 * Memory region flag telling that the region has the same virtual and physical
 * addresses.
 */
# define MEM_REGION_FLAG_IDENTITY	0b100000

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

	/* Pointer to the end of the memory */
	void *memory_end;
	/* Pointer to the beginning of allocatable memory */
	void *heap_begin;
	/* Pointer to the end of allocatable memory */
	void *heap_end;
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

typedef struct mem_space mem_space_t;

/*
 * Structure representing a memory region in the memory space. (Used addresses)
 */
typedef struct mem_region
{
	/* Linked list of memory regions in the current memory space. */
	list_head_t list;
	/*
	 * Double-linked list of memory regions that share the same physical space.
	 * Elements in this list might not be in the same memory space.
	 */
	list_head_t shared_list;
	/* The node of the tree the structure is stored in. */
	avl_tree_t node;
	/* The memory space associated with the region. */
	mem_space_t *mem_space;

	/* The flags for the memory region. */
	char flags;
	/* The beginning address of the region. */
	void *begin;
	/* The size of the region in pages. */
	size_t pages;
	/* The number of used pages in the region. */
	size_t used_pages;
} mem_region_t;

/*
 * Structure representing a memory gap int the memory space. (Free addresses)
 */
typedef struct mem_gap
{
	/* Double-linked list of memory gaps in the current memory space. */
	list_head_t list;
	/* The node of the tree the structure is stored in */
	avl_tree_t node;
	/* The memory space associated with the gap. */
	mem_space_t *mem_space;

	/* The beginning address of the gap. */
	void *begin;
	/* The size of the gap in pages. */
	size_t pages;
} mem_gap_t;

/*
 * The object used in x86 memory permissions handling.
 */
typedef uint32_t *vmem_t;

/*
 * Structure representing a memory context. Allowing to allocate virtual memory.
 */
struct mem_space
{
	/* Linked list of regions (used zones) */
	list_head_t *regions;
	/* Linked list of gaps (free zones, ordered by growing pointer) */
	list_head_t *gaps;
	/* Binary tree of regions (ordered by pointer) */
	avl_tree_t *used_tree;
	/* Binary tree of gaps (ordered by size in pages) */
	avl_tree_t *free_tree;

	/* The spinlock for this memory space. */
	spinlock_t spinlock;

	/* An architecture dependent object to handle memory permissions. */
	vmem_t page_dir;
};

extern memory_info_t mem_info;
extern vmem_t kernel_vmem;

extern int check_a20(void);
void enable_a20(void);

void memmap_init(void *multiboot_ptr, void *kernel_end);
void memmap_print(void);
const char *memmap_type(uint32_t type);

void print_mem_amount(size_t amount);
void *clone_page(void *ptr);

void get_memory_usage(mem_usage_t *usage);
# ifdef KERNEL_DEBUG
void print_mem_usage(void);
# endif

mem_space_t *mem_space_init(void);
mem_space_t *mem_space_clone(mem_space_t *space);
// TODO Allocation at a given address
void *mem_space_alloc(mem_space_t *space, size_t pages, int flags);
void *mem_space_alloc_fixed(mem_space_t *space, void *addr, size_t pages,
	int flags);
int mem_space_free(mem_space_t *space, void *ptr, size_t pages);
int mem_space_free_stack(mem_space_t *space, void *stack);
int mem_space_can_access(mem_space_t *space, const void *ptr, size_t size,
	int write);
int mem_space_handle_page_fault(mem_space_t *space, void *ptr, int error_code);
void mem_space_destroy(mem_space_t *space);

vmem_t vmem_init(void);
void vmem_kernel(void);
uint32_t *vmem_resolve(vmem_t vmem, void *ptr);
int vmem_is_mapped(vmem_t vmem, void *ptr);
void vmem_map(vmem_t vmem, void *physaddr, void *virtaddr, int flags);
void vmem_map_range(vmem_t vmem, void *physaddr, void *virtaddr,
	size_t pages, int flags);
void vmem_identity(vmem_t vmem, void *page, int flags);
void vmem_identity_range(vmem_t vmem, void *from, size_t pages, int flags);
void vmem_unmap(vmem_t vmem, void *virtaddr);
void vmem_unmap_range(vmem_t vmem, void *virtaddr, size_t pages);
int vmem_contains(vmem_t vmem, const void *ptr, size_t size);
void *vmem_translate(vmem_t vmem, void *ptr);
uint32_t vmem_get_entry(vmem_t vmem, void *ptr);
uint32_t vmem_page_flags(vmem_t vmem, void *ptr);
vmem_t vmem_clone(vmem_t vmem);
void vmem_flush(vmem_t vmem);
void vmem_destroy(vmem_t vmem);

extern void paging_enable(vmem_t vmem);
extern void tlb_reload(void);
extern void *cr2_get(void);
extern void *cr3_get(void);
extern void paging_disable(void);

#endif
