#ifndef MEMORY_H
# define MEMORY_H

# include <kernel.h>
# include <memory/buddy/buddy.h>
# include <memory/pages/pages.h>
# include <memory/kmalloc/kmalloc.h>
# include <memory/slab/slab.h>
# include <util/util.h>

# define PAGE_SIZE		0x1000
# define KERNEL_BEGIN	((void *) 0x100000)

# define PAGING_TABLE_PAGE_SIZE		0b10000000
# define PAGING_TABLE_ACCESSED		0b00100000
# define PAGING_TABLE_CACHE_DISABLE	0b00010000
# define PAGING_TABLE_WRITE_THROUGH	0b00001000
# define PAGING_TABLE_USER			0b00000100
# define PAGING_TABLE_WRITE			0b00000010
# define PAGING_TABLE_PRESENT		0b00000001

# define PAGING_PAGE_GLOBAL			0b100000000
# define PAGING_PAGE_DIRTY			0b001000000
# define PAGING_PAGE_ACCESSED		0b000100000
# define PAGING_PAGE_CACHE_DISABLE	0b000010000
# define PAGING_PAGE_WRITE_THROUGH	0b000001000
# define PAGING_PAGE_USER			0b000000100
# define PAGING_PAGE_WRITE			0b000000010
# define PAGING_PAGE_PRESENT		0b000000001

# define PAGING_FLAGS_MASK	0b111111111111
# define PAGING_ADDR_MASK	~((uint32_t) PAGING_FLAGS_MASK)

# define PAGETOPTR(page)	((void *) (page) * PAGE_SIZE)
# define PTRTOPAGE(ptr)		((uintptr_t) (ptr) / PAGE_SIZE)

# define ADDR_TABLE(addr)	(((uintptr_t) (addr) >> 22) & 0x3ff)
# define ADDR_PAGE(addr)	(((uintptr_t) (addr) >> 12) & 0x3ff)
# define ADDR_REMAIN(addr)	((uintptr_t) (addr) & 0xfff)

typedef uint32_t *vmem_t;

typedef struct
{
	size_t reserved;
	size_t system;
	size_t allocated;
	size_t swap;
	size_t free;
} mem_usage_t;

extern size_t memory_maps_size;
extern size_t memory_maps_entry_size;
extern void *memory_maps;

extern void *memory_end;
extern void *heap_begin, *heap_end;
extern size_t available_memory;

extern vmem_t kernel_vmem;

extern bool check_a20(void);
void enable_a20(void);

void memmap_init(const boot_info_t *info, void *kernel_end);
void memmap_print(void);
const char *memmap_type(uint32_t type);

void *clone_page(void *ptr);

vmem_t vmem_init(void);
void vmem_kernel(void);
void vmem_identity(vmem_t vmem, void *page, int flags);
void vmem_identity_range(vmem_t vmem, void *from, void *to, int flags);
void vmem_map(vmem_t vmem, void *physaddr, void *virtaddr, int flags);
vmem_t vmem_clone(vmem_t vmem, bool mem_dup);
void *vmem_translate(vmem_t vmem, void *ptr);
bool vmem_contains(vmem_t vmem, const void *ptr, size_t size);
void *vmem_alloc_pages(vmem_t vmem, size_t pages);
void vmem_free_pages(vmem_t vmem, size_t pages, bool mem_free);
void vmem_free(vmem_t vmem, bool mem_free);

extern void paging_enable(vmem_t vmem);
extern void tlb_reload(void);
extern void *cr2_get(void);
extern void paging_disable(void);

void get_memory_usage(mem_usage_t *usage);

#endif
