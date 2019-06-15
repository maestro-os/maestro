#ifndef MEMORY_H
# define MEMORY_H

# include "../kernel.h"
# include "buddy/buddy.h"
# include "slab/slab.h"
# include "../util/util.h"

# define PAGE_SIZE		0x1000

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

# define PAGING_DIRECTORY_SIZE	0x400
# define PAGING_TABLE_SIZE		0x400
# define PAGING_TOTAL_PAGES		(PAGING_DIRECTORY_SIZE * PAGING_TABLE_SIZE)

# define PAGETOPTR(page)	((void *) (page) * PAGE_SIZE)
# define PTRTOPAGE(ptr)		((uintptr_t) (ptr) / PAGE_SIZE)

typedef uint32_t block_order_t;
typedef uint32_t *vmem_t;

void *heap_begin, *heap_end;
size_t available_memory;

size_t memory_maps_count;
multiboot_mmap_entry_t *memory_maps;

extern bool check_a20(void);
void enable_a20(void);

const char *memmap_type(const uint32_t type);

block_order_t buddy_get_order(const size_t size);
void *buddy_alloc(const block_order_t order);
void *buddy_alloc_zero(const block_order_t order);
void buddy_free(void *ptr);

void *clone_page(void *ptr);

void *kmalloc(const size_t size);
void *kmalloc_zero(const size_t size);
void *krealloc(void *ptr, const size_t size);
void kfree(void *ptr);

vmem_t vmem_init(void);
vmem_t vmem_clone(vmem_t vmem, const bool mem_dup);
void *vmem_translate(vmem_t vmem, void *ptr);
void *vmem_alloc_pages(vmem_t vmem, const size_t pages);
void vmem_free_pages(vmem_t vmem, const size_t pages, const bool mem_free);
void vmem_free(vmem_t vmem, const bool mem_free);

extern void paging_enable(const vmem_t vmem);
extern void tlb_reload(void);
extern void paging_disable(void);

#endif
