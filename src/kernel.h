#ifndef KERNEL_H
# define KERNEL_H

# include "multiboot.h"
# include "libc/string.h"

# define KERNEL_VERSION	"0.1"

# define KERNEL_MAGIC

# define GD_NULL	0

# define GD_LIMIT_MASK		0x10
# define GD_LIMIT_MASK_2	(0x4 >> GD_LIMIT_MASK)
# define GD_BASE_MASK		0x18
# define GD_BASE_MASK_2		(0x8 >> GD_BASE_MASK)

# define GD_LIMIT_OFFSET	0
# define GD_LIMIT_OFFSET_2	(6 * 8) // + 4?
# define GD_BASE_OFFSET		(2 * 8)
# define GD_BASE_OFFSET_2	(7 * 8)
# define GD_ACCESS_OFFSET	(5 * 8)
# define GD_FLAGS_OFFSET	((6 * 8) + 4)

# define GD_ACCESS_BASE					0b00001

# define GD_ACCESS_ACCESSED				0b1
# define GD_ACCESS_READABLE				0b01
# define GD_ACCESS_WRITABLE				0b01
# define GD_ACCESS_CONFORMING			0b001
# define GD_ACCESS_DOWNWARD_EXPENSION	0b001
# define GD_ACCESS_UPWARD_EXPENSION		0
# define GD_ACCESS_EXECUTABLE			0b0001

# define GD_ACCESS_PRIVILEGE_RING_0		0b0000000
# define GD_ACCESS_PRIVILEGE_RING_1		0b0000001
# define GD_ACCESS_PRIVILEGE_RING_2		0b0000010
# define GD_ACCESS_PRIVILEGE_RING_3		0b0000011

# define GD_ACCESS_PRESENT				0b00000001

# define GD_FLAGS_NYBBLE_DEFAULT_SIZE_16BITS	0
# define GD_FLAGS_NYBBLE_DEFAULT_SIZE_32BITS	0b0000001

# define GD_FLAGS_NYBBLE_GRANULARITY_4K			0b00000001

# define KERNEL_HEAP_BEGIN	(void *) 0x200000
# define KERNEL_HEAP_SIZE	0x100000
# define MEM_PAGE_SIZE		0x1000

# define MEM_STATE_FREE		0
# define MEM_STATE_USED		0b01
# define MEM_STATE_HEADER	0b10

typedef struct gdt
{
	uint16_t size;
	const uint32_t offset;
} gdt_t;

typedef uint64_t global_descriptor_t;

typedef enum mem_state
{
	MEM_FREE,
	MEM_USED
} mem_state_t;

typedef struct mem_node
{
	mem_state_t state;
	size_t size;

	struct mem_node	*next;
} mem_node_t;

void *memory_end;

void mm_init();
void *mm_find_free(void *ptr, size_t size);
void mm_free(void *ptr);

uint8_t inb(const uint16_t port);
void outb(const uint16_t port, const uint8_t value);

__attribute__((noreturn))
void panic(const char *reason);

__attribute__((noreturn))
void kernel_halt();

#endif
