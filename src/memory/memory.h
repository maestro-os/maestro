#ifndef MEMORY_H
# define MEMORY_H

# include "../kernel.h"
# include "../util/util.h"

# define PAGE_SIZE		0x100
// TODO Use pages instead of blocks?
# define BLOCK_SIZE		0x1000
# define HEAP_BEGIN		((void *) 0x400000)
# define BUDDY_MIN_SIZE	0x100000

# define MAX_BUDDY_NODES(order)	(pow2((order) + 1) - 1)
// TODO Add informations for every page?
# define ALLOC_META_SIZE(order)	(UPPER_DIVISION(MAX_BUDDY_NODES(order)\
	* sizeof(buddy_state_t), 8))

# define ORDERTOSIZE(order)					(BLOCK_SIZE * pow2(order))
# define BUDDY_INDEX(max_order, order, i)	((max_order) - (order) + (i))

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

typedef size_t buddy_order_t;
typedef int16_t buddy_state_t;

typedef struct buddy_alloc
{
	void *begin;
	size_t size;

	buddy_state_t *states;

	struct buddy_alloc *next;
} buddy_alloc_t;

buddy_alloc_t *allocators;

void buddy_init();
buddy_order_t alloc_max_order(const buddy_alloc_t *alloc);

void buddy_reserve_blocks(const size_t count);
void *buddy_alloc(const size_t pages);
void buddy_free(const void *ptr, const size_t pages);

typedef uint16_t paging_flags_t;

void *paging_alloc(uint32_t *directory, void *hint,
	const size_t length, const paging_flags_t flags);
void paging_free(uint32_t *directory, void *ptr, const size_t length);
uint32_t *paging_get_page(const uint32_t *directory, const size_t page);
void paging_set_page(uint32_t *directory, const size_t page,
	void *physaddr, const paging_flags_t flags);

extern void paging_enable(const uint32_t *directory);
extern void paging_disable();

typedef uint8_t kmalloc_flags_t;

void *kmalloc(const size_t size);
void *krealloc(void *ptr, const size_t size);
void kfree(void *ptr);

#endif
