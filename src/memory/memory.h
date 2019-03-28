#ifndef MEMORY_H
# define MEMORY_H

# include "../kernel.h"

# define KALLOC_ALIGNED	0b01
# define KALLOC_FAST	0b10

# define PAGE_SIZE			0x1000
# define BLOCK_SIZE			0x10000
# define PAGES_PER_BLOCK	(BLOCK_SIZE / PAGE_SIZE)

typedef uint8_t kmalloc_flags;

void *memory_end;

extern bool check_a20();
void enable_a20();

// TODO

#endif
