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
 * Returns the id of the block from the pointer to its state.
 */
# define BLOCK_ID(state_ptr)	((uintptr_t) ((state_ptr) - blocks_states)\
	/ sizeof(block_state_t))

/*
 * Returns the pointer to the page frame from the given id.
 */
# define BLOCK_ID_PTR(id)		(buddy_begin + (id) * PAGE_SIZE)

/*
 * Returns the id to the page frame from the given pointer.
 */
# define BLOCK_PTR_ID(ptr)		((uintptr_t) ((ptr) - buddy_begin) / PAGE_SIZE)

/*
 * Returns the buddy for the given block with the given order.
 */
# define GET_BUDDY(id, order)	((id) ^ (order))

/*
 * The type representing the state of a block. Value BLOCK_STATE_USED means used
 * block. Any other value means free block and represents the identifier of the
 * next block into the free list.
 */
typedef uint32_t block_state_t;

#endif
