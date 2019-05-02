#ifndef MEMORY_H
# define MEMORY_H

# include "../kernel.h"
# include "buddy.h"

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

void *memory_end;

extern bool check_a20();
void enable_a20();

void buddy_init();
void buddy_set_block(const size_t i, const size_t order, const int used);
void *buddy_alloc(const size_t order);
void buddy_free(void *ptr);

// TODO vmalloc, etc...

extern void paging_enable(const uint32_t *directory);
extern void paging_disable();

void *kmalloc(const size_t size);
void *krealloc(void *ptr, const size_t size);
void kfree(void *ptr);

#endif
