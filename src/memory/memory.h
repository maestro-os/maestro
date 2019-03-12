#ifndef MEMORY_H
# define MEMORY_H

# include "../kernel.h"

# define GD_NULL	0

# define GD_LIMIT_MASK		0x0ffff
# define GD_LIMIT_MASK_2	0xf0000
# define GD_BASE_MASK		0x0000ffff
# define GD_BASE_MASK_2		0x00ff0000
# define GD_BASE_MASK_3		0xff000000

# define GD_LIMIT_SHIFT_2	0x20
# define GD_BASE_SHIFT_2	0x20
# define GD_BASE_SHIFT_3	0x30

# define GD_LIMIT_OFFSET	0x0
# define GD_BASE_OFFSET		0x10
# define GD_BASE_OFFSET_2	0x20
# define GD_ACCESS_OFFSET	0x28
# define GD_LIMIT_OFFSET_2	0x30
# define GD_FLAGS_OFFSET	0x34
# define GD_BASE_OFFSET_3	0x38

# define GD_ACCESS_BASE					0b10000000
# define GD_ACCESS_PRIVILEGE_RING_0		0b00000000
# define GD_ACCESS_PRIVILEGE_RING_1		0b00100000
# define GD_ACCESS_PRIVILEGE_RING_2		0b01000000
# define GD_ACCESS_PRIVILEGE_RING_3		0b01100000
# define GD_ACCESS_S					0b00010000
# define GD_ACCESS_EXECUTABLE			0b00001000
# define GD_ACCESS_DOWNWARD_EXPENSION	0b00000100
# define GD_ACCESS_UPWARD_EXPENSION		0b00000000
# define GD_ACCESS_CONFORMING			0b00000100
# define GD_ACCESS_READABLE				0b00000010
# define GD_ACCESS_WRITABLE				0b00000010

# define GD_FLAGS_GRANULARITY_4K	0b1000
# define GD_FLAGS_SIZE_16BITS		0b0000
# define GD_FLAGS_SIZE_32BITS		0b0100

# define PAGING_DIR_PAGE_SIZE		0b10000010
# define PAGING_DIR_ACCESSED		0b00100000
# define PAGING_DIR_CACHE_DISABLE	0b00010000
# define PAGING_DIR_WRITE_THROUGH	0b00001000
# define PAGING_DIR_USER			0b00000100
# define PAGING_DIR_WRITE			0b00000010
# define PAGING_DIR_PRESENT			0b00000001

# define PAGING_TABLE_GLOBAL		0b100000000
# define PAGING_TABLE_DIRTY			0b001000000
# define PAGING_TABLE_ACCESSED		0b000100000
# define PAGING_TABLE_CACHE_DISABLE	0b000010000
# define PAGING_TABLE_WRITE_THROUGH	0b000001000
# define PAGING_TABLE_USER			0b000000100
# define PAGING_TABLE_WRITE			0b000000010
# define PAGING_TABLE_PRESENT		0b000000001

# define PAGE_SIZE			0x1000
# define MEMORY_BLOCK_SIZE	0x10000
# define KERNEL_RESERVED	((void *) 0x200000)

# define FREE_BLOCK_PID		(~((pid_t) 0))

# define MEM_STATE_FREE		0
# define MEM_STATE_USED		0b01
# define MEM_STATE_HEADER	0b10

typedef struct gdt
{
	uint16_t size;
	uint32_t offset;
} gdt_t;

typedef uint64_t global_descriptor_t;

void *memory_end;

typedef struct page
{
	size_t directory_entry;
	size_t table_entry;

	pid_t owner;
} page_t;

typedef struct mem_node
{
	page_t *page;

	struct mem_list *left;
	struct mem_list *right;
} mem_list_t;

extern int check_a20();
void enable_a20();

void paging_init();

void *paging_get_addr(const page_t *page);
page_t paging_get_page(const void *addr);

uint32_t *paging_get_table(const size_t i);
void paging_set_table(const size_t i, const uint32_t *table,
	const uint16_t flags);

void mm_init();

size_t mm_required_pages(const size_t length);
page_t *mm_alloc_pages(const pid_t pid, void *hint, const size_t count);

void mm_free(void *addr);

#endif
