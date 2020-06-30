#ifndef BUDDY_INTERNAL_H
# define BUDDY_INTERNAL_H

# include <memory/buddy/buddy.h>
# include <util/util.h>

/*
 * The state of a used block. This value cannot be reached thanks to the
 * capacity of the pointer type (because the value is the page identifier, not
 * the pointer).
 */
# define BLOCK_STATE_USED	((block_state_t) -1)

/*
 * The type representing the state of a block. Value BLOCK_STATE_USED means used
 * block. Any other value means free block and represents the identifier of the
 * next block into the free list.
 */
typedef uint32_t block_state_t;

#endif
