#ifndef MEMORY_H
# define MEMORY_H

# include "../kernel.h"

# define PAGE_SIZE			0x1000
# define BLOCK_SIZE			0x10000
# define PAGES_PER_BLOCK	(BLOCK_SIZE / PAGE_SIZE)

# define HEAP_BEGIN ((void *) 0x400000)

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

typedef uint16_t paging_flags_t;
typedef uint8_t kmalloc_flags_t;

void *memory_end;

extern bool check_a20();
void enable_a20();

typedef enum
{
	FREE,
	RESERVED,
	USED
} buddy_state_t;

typedef struct
{
	buddy_state_t state;
	void *ptr;
	size_t size;
} buddy_t;

void buddy_init();
buddy_t *buddy_get(void *ptr);
buddy_t *buddy_alloc(const size_t blocks);
void buddy_free(buddy_t *buddy);

inline void *page_to_ptr(const size_t page)
{
	return (void *) (page * PAGE_SIZE);
}

inline size_t ptr_to_page(const void *ptr)
{
	return (uintptr_t) ptr / PAGE_SIZE;
}

void *paging_alloc(uint32_t *directory, void *hint,
	const size_t length, const paging_flags_t flags);
void paging_free(uint32_t *directory, void *ptr, const size_t length);
uint32_t *paging_get_page(const uint32_t *directory, const size_t page);
void paging_set_page(uint32_t *directory, const size_t page,
	void *physaddr, const paging_flags_t flags);

extern void paging_enable(const uint32_t *directory);
extern void paging_disable();

void *kmalloc(const size_t size);
void *krealloc(void *ptr, const size_t size);
void kfree(void *ptr);

#endif
