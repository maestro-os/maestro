#include <memory/buddy/buddy_internal.h>

/*
 * This file handles free lists for the buddy allocators.
 */

/*
 * Links the given element to the given free list.
 */
void free_list_push(zone_t *zone, frame_order_t order,
	frame_state_t *s)
{
	debug_assert(order <= BUDDY_MAX_ORDER && s, "buddy: invalid arguments");
	s->prev = FRAME_ID(zone, s);
	s->next = (zone->free_list[order] ? FRAME_ID(zone, zone->free_list[order])
		: FRAME_ID(zone, s));
	s->order = order;
	zone->states[s->next].prev = FRAME_ID(zone, s);
	debug_assert(&zone->states[s->prev] == s, "buddy: free list failure");
	debug_check_free_frame(zone, s);
	zone->free_list[order] = s;
}

/*
 * Unlinks the first element of the given free list.
 */
void free_list_pop(zone_t *zone, frame_order_t order)
{
	frame_state_t *s;
	size_t frame_id;

	debug_assert(order <= BUDDY_MAX_ORDER, "buddy: invalid argument");
	s = zone->free_list[order];
	debug_check_free_frame(zone, s);
	zone->free_list[order] = &zone->states[s->next];
	frame_id = FRAME_ID(zone, s);
	zone->states[s->prev].next = (s->next != frame_id ? s->next : s->prev);
	zone->states[s->next].prev = (s->prev != frame_id ? s->prev : s->next);
	s->prev = FRAME_STATE_USED;
	if(FRAME_IS_USED(zone->free_list[order]))
		zone->free_list[order] = NULL;
}

/*
 * Unlinks the given element of the given free list.
 */
void free_list_remove(zone_t *zone, frame_order_t order, frame_state_t *s)
{
	size_t frame_id;

	debug_assert(order <= BUDDY_MAX_ORDER && s, "buddy: invalid arguments");
	debug_assert(zone->free_list[order], "buddy: empty free list");
	debug_check_free_frame(zone, s);
	frame_id = FRAME_ID(zone, s);
	if(zone->free_list[order] == s)
	{
		debug_assert(s->prev == frame_id, "buddy: invalid free list");
		zone->free_list[order] = (s->next == frame_id ? NULL
			: &zone->states[s->next]);
	}
	zone->states[s->prev].next = (s->next != frame_id ? s->next : s->prev);
	zone->states[s->next].prev = (s->prev != frame_id ? s->prev : s->next);
	s->prev = FRAME_STATE_USED;
}

/*
 * Splits the given frame from order `from` to order `to`.
 */
void free_list_split(zone_t *zone, const frame_order_t from,
	const frame_order_t to)
{
	frame_state_t *s;
	size_t frame_id, i;

	debug_assert(from <= BUDDY_MAX_ORDER && to < from, "buddy: invalid orders");
	s = zone->free_list[from];
	frame_id = FRAME_ID(zone, s);
	free_list_pop(zone, from);
	free_list_push(zone, to, s);
	for(i = to; i < from; ++i)
		free_list_push(zone, i, &zone->states[GET_BUDDY(frame_id, i)]);
}

/*
 * Coalesces the given frame from the given order.
 */
void free_list_coalesce(zone_t *zone, frame_state_t *b,
	const frame_order_t order)
{
	size_t i, frame_id, buddy;
	frame_state_t *buddy_state;

	debug_assert(b, "buddy: invalid argument");
	debug_assert(order <= BUDDY_MAX_ORDER, "buddy: invalid order");
	i = order;
	while(i < BUDDY_MAX_ORDER)
	{
		debug_assert(!FRAME_IS_USED(b), "buddy: trying to coalesce used frame");
		frame_id = FRAME_ID(zone, b);
		if(frame_id >= zone->pages || frame_id + POW2(i) > zone->pages)
			break;
		buddy = GET_BUDDY(frame_id, i);
		if(buddy >= zone->pages || buddy + POW2(i) > zone->pages)
			break;
		buddy_state = &zone->states[buddy];
		if(buddy_state->order != i)
			break;
		debug_assert(b != buddy_state, "buddy: invalid buddy");
		if(FRAME_IS_USED(buddy_state))
			break;
		free_list_remove(zone, i, b);
		free_list_remove(zone, i, buddy_state);
		b = MIN(b, buddy_state);
		free_list_push(zone, ++i, b);
	}
}
