#ifndef BUDDY_INTERNAL_H
# define BUDDY_INTERNAL_H

# include <memory/buddy/buddy.h>
# include <util/util.h>

/*
 * The state of a used block. This value cannot be reached thanks to the
 * capacity of the pointer type (because the value is the page identifier, not
 * the pointer).
 */
# define FRAME_STATE_USED	((uint32_t) -1)

/*
 * Tells if the specified block is used.
 */
# define FRAME_IS_USED(state_ptr)	((state_ptr)->prev == FRAME_STATE_USED\
 	|| (state_ptr)->next == FRAME_STATE_USED)

/*
 * Returns the pointer to the frame state for the given frame id.
 */
# define FRAME_STATE_GET(id)	(&frames_states[(id)])

/*
 * Returns the id of the block from the pointer to its state.
 */
# define FRAME_ID(state_ptr)\
 	(((uintptr_t) (state_ptr) - (uintptr_t) frames_states)\
		/ sizeof(frame_state_t))

/*
 * Returns the pointer to the page frame from the given id.
 */
# define FRAME_ID_PTR(id)		(buddy_begin + (id) * PAGE_SIZE)

/*
 * Returns the id to the page frame from the given pointer.
 */
# define FRAME_PTR_ID(ptr)		((uintptr_t) ((ptr) - buddy_begin) / PAGE_SIZE)

/*
 * Returns the buddy for the given block with the given order.
 */
# define GET_BUDDY(id, order)	((id) ^ POW2(order))

# ifdef KERNEL_DEBUG
#  include <debug/debug.h>

/*
 * Asserts that the given free frame is valid.
 */
#  define debug_check_free_frame(state)\
	do\
	{\
		debug_assert((uintptr_t) (state) >= (uintptr_t) frames_states\
			&& (uintptr_t) (state) < ((uintptr_t) frames_states\
				+ pages_count * sizeof(frame_state_t)),\
					"buddy: invalid free frame");\
		debug_assert(!FRAME_IS_USED(state), "buddy: free frame is used");\
		debug_assert(!FRAME_IS_USED(FRAME_STATE_GET((state)->prev)),\
			"buddy: previous free frame is used");\
		debug_assert(!FRAME_IS_USED(FRAME_STATE_GET((state)->next)),\
			"buddy: next free frame is used");\
	}\
	while(0)

/*
 * Asserts that the given block of memory is valid.
 */
#  define debug_check_block(begin, ptr, order)\
	debug_assert(sanity_check(ptr)\
		&& IS_ALIGNED((ptr) - (begin), PAGE_SIZE << (order))\
		&& (void *) (ptr) >= mem_info.heap_begin\
		&& (void *) (ptr) < mem_info.heap_end, "buddy: invalid block")

/*
 * Asserts that the given order is valid.
 */
#  define debug_check_order(order)\
     debug_assert((order) <= BUDDY_MAX_ORDER, "buddy: invalid order")
# else
#  define debug_check_free_frame(frame)
#  define debug_check_block(ptr)
#  define debug_check_order(order)
# endif

/*
 * Structure representing the state of page frame. FRAME_IS_USED allows to check
 * if the block is used.
 * A frame pointing to itself represtents the end of the list.
 */
typedef struct
{
	/* Id of the previous page frame in the free list. */
	uint32_t prev;
	/* Id of the next page frame in the free list. */
	uint32_t next;
} frame_state_t;

#endif
