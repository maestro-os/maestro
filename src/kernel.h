#ifndef KERNEL_H
# define KERNEL_H

# include "multiboot.h"
# include "libc/string.h"

# define KERNEL_VERSION	"0.1"

# define KERNEL_MAGIC

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

# define KERNEL_HEAP_BEGIN	((void *) 0x200000)
# define KERNEL_HEAP_SIZE	0x100000
# define MEM_PAGE_SIZE		0x1000

# define MEM_STATE_FREE		0
# define MEM_STATE_USED		0b01
# define MEM_STATE_HEADER	0b10

typedef struct gdt
{
	uint16_t size;
	uint32_t offset;
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

extern int check_a20();
void enable_a20();

void mm_init();
void *mm_find_free(void *ptr, size_t size);
void mm_free(void *ptr);

uint8_t inb(const uint16_t port);
void outb(const uint16_t port, const uint8_t value);

void error_handler(const int error);

__attribute__((noreturn))
void panic(const char *reason);

__attribute__((noreturn))
void kernel_halt();

#endif
