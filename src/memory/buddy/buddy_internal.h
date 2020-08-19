#ifndef BUDDY_INTERNAL_H
# define BUDDY_INTERNAL_H

# include <memory/buddy/buddy.h>
# include <util/util.h>

/*
 * The state of a used frame. This value cannot be reached thanks to the
 * capacity of the pointer type (because the value is the page identifier, not
 * the pointer).
 */
# define FRAME_STATE_USED	((uint32_t) -1)

/*
 * Tells if the specified frame is used.
 */
# define FRAME_IS_USED(state_ptr)	((state_ptr)->prev == FRAME_STATE_USED)

/*
 * Returns the id of the frame from the pointer to its state.
 */
# define FRAME_ID(zone, state_ptr)\
 	(((uintptr_t) (state_ptr) - (uintptr_t) (zone)->states)\
		/ sizeof(frame_state_t))

/*
 * Returns the pointer to the page frame from the given id.
 */
# define FRAME_ID_PTR(zone, id)	((zone)->begin + (id) * PAGE_SIZE)

/*
 * Returns the id to the page frame from the given pointer.
 */
# define FRAME_PTR_ID(zone, ptr)\
	((uintptr_t) ((ptr) - (zone)->begin) / PAGE_SIZE)

/*
 * Returns the buddy for the given frame with the given order.
 */
# define GET_BUDDY(id, order)	((id) ^ POW2(order))

# ifdef KERNEL_DEBUG
/*
 * Asserts that the given free frame is valid.
 */
#  define debug_check_free_frame(zone, state)\
	do\
	{\
		debug_assert((uintptr_t) (state) >= (uintptr_t) zone->states\
			&& (uintptr_t) (state) < ((uintptr_t) zone->states\
				+ zone->pages * sizeof(frame_state_t)),\
					"buddy: invalid free frame");\
		debug_assert(!FRAME_IS_USED(state), "buddy: free frame is used");\
		debug_assert(!FRAME_IS_USED(&zone->states[(state)->prev]),\
			"buddy: previous free frame is used");\
		debug_assert(!FRAME_IS_USED(&zone->states[(state)->next]),\
			"buddy: next free frame is used");\
	}\
	while(0)

/*
 * Asserts that the given frame of memory is valid.
 */
// TODO Check if end of frame is lower than beginning and still in range
#  define debug_check_frame(zone, ptr, order)\
	debug_assert(sanity_check(ptr)\
		&& IS_ALIGNED((ptr) - (zone)->begin, PAGE_SIZE << (order))\
		&& (void *) (ptr) >= (zone)->begin\
		&& (void *) (ptr) < (zone)->begin + (zone)->pages * PAGE_SIZE,\
			"buddy: invalid frame")

/*
 * Asserts that the given order is valid.
 */
#  define debug_check_order(order)\
     debug_assert((order) <= BUDDY_MAX_ORDER, "buddy: invalid order")
# else
#  define debug_check_free_frame(frame)
#  define debug_check_frame(begin, ptr, order)
#  define debug_check_order(order)
# endif

/*
 * Structure representing the state of page frame. FRAME_IS_USED allows to check
 * if the frame is used.
 * A frame pointing to itself represtents the end of the list.
 */
typedef struct
{
	/* Id of the previous page frame in the free list. */
	uint32_t prev;
	/* Id of the next page frame in the free list. */
	uint32_t next;
	/* Order of the current frame. */
	frame_order_t order;
} frame_state_t;

/*
 * A zone for the buddy allocator.
 */
typedef struct
{
	/* The linked list containing other zones of the same type */
	list_head_t list;
	/* The type of the zone of memory, represented by a buddy allocator flag */
	int type;

	/* The beginning of the memory zone */
	void *begin;
	/* The size of the memory zone in pages */
	size_t pages;
	/* The available pages count in the zone */
	size_t available;

	/*
	 * The list of linked lists containing free frames, sorted according to
	 * frames' order.
	 */
	frame_state_t *free_list[BUDDY_MAX_ORDER + 1];
	/* The array of states for frames in the zone */
	frame_state_t states[0];
} zone_t;

# ifdef KERNEL_DEBUG
void buddy_free_list_check(void);
void buddy_free_list_print(void);
int buddy_free_list_has(frame_state_t *state);
int buddy_free_list_has_(frame_state_t *state, frame_order_t order);
# endif

void free_list_push(zone_t *zone, frame_order_t order, frame_state_t *s);
void free_list_pop(zone_t *zone, frame_order_t order);
void free_list_remove(zone_t *zone, frame_order_t order, frame_state_t *s);
void free_list_split(zone_t *zone, const frame_order_t from,
	const frame_order_t to);
void free_list_coalesce(zone_t *zone, frame_state_t *b,
	const frame_order_t order);

void zone_init(zone_t *zone, int type, void *begin, size_t pages);
zone_t *zone_get(frame_order_t order, int type);
zone_t *zone_get_for(void *ptr);

#endif
