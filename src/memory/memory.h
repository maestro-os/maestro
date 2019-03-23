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

# define PAGING_TABLE_PAGE_SIZE		0b10000010
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

# define TABLES_COUNT		0x400
# define PAGES_PER_TABLE	0x400

# define PAGE_SIZE				0x1000
# define KERNEL_RESERVED		((void *) (PAGE_SIZE * PAGES_PER_TABLE))

# define PAGES_ADDR	0x400000

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

extern int check_a20();
void enable_a20();

void paging_init();

void *paging_alloc(const void *hint, const size_t count, const uint16_t flags);
void paging_free(const void *page, const size_t count);

void mm_init();

void *kmalloc(const size_t size);
void kfree(void *ptr);

size_t mm_required_pages(const size_t length);
// TODO Memory allocation and free

#endif
