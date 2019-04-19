#ifndef BUDDY_H
# define BUDDY_H

# define PAGE_SIZE			0x1000
# define BLOCK_SIZE			0x10000
# define PAGES_PER_BLOCK	(BLOCK_SIZE / PAGE_SIZE)

# define HEAP_BEGIN ((void *) 0x400000)

# if HEAP_BEGIN < BLOCK_SIZE
#  error "BLOCK_SIZE must be lower than HEAP_BEGIN!"
# endif

typedef enum
{
	FREE,
	RESERVED,
	USED
} buddy_state_t;

typedef struct
{
	buddy_state_t state;
	int order;
} buddy_t;

#endif
