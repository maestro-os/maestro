#include <memory/buddy/buddy_internal.h>

/*
 * This file handles buddy allocator zones, which allow to reserve portions of
 * the physical memory for specific usages.
 *
 * There exist three types of zones:
 * - DMA: reserved for Direct Memory Access
 * - Kernel: reserved for the kernel
 * - User: reserved for processes
 */


/*
 * The list of DMA allocation zones.
 */
static list_head_t *dma_zone;
/*
 * The list of kernel allocation zones.
 */
static list_head_t *kernel_zone;
/*
 * The list of user allocation zones.
 */
static list_head_t *user_zone;

/*
 * Returns the list for the given zone `type`.
 */
static list_head_t **get_list(int type)
{
	switch(type)
	{
		case BUDDY_FLAG_ZONE_DMA: return &dma_zone;
		case BUDDY_FLAG_ZONE_KERNEL: return &kernel_zone;
		case BUDDY_FLAG_ZONE_USER: return &user_zone;
	}
	return NULL;
}

/*
 * Initializes the given zone.
 */
void zone_init(zone_t *zone, int type, void *begin, size_t pages)
{
	size_t i, order;
	frame_state_t *s;

	debug_assert(sanity_check(zone) && sanity_check(begin),
		"zone: invalid argument");
	bzero(zone, sizeof(zone_t));
	zone->type = type;
	zone->begin = begin;
	zone->pages = pages;
	zone->available = zone->pages;
	memset((void *) zone->states, FRAME_STATE_USED,
		zone->pages * sizeof(frame_state_t));
	bzero(&zone->free_list, sizeof(zone->free_list));
	for(i = 0, order = BUDDY_MAX_ORDER; i < zone->pages; i += POW2(order))
	{
		while(order > 0 && i + POW2(order) > zone->pages)
			--order;
		if(i >= zone->pages)
			break;
		s = &zone->states[i];
		debug_assert((uintptr_t) s < (uintptr_t) (zone->states
			+ zone->pages * sizeof(frame_state_t)),
			"buddy: frame state out of bounds");
		free_list_push(zone, order, s);
	}
	list_insert_front(get_list(type), &zone->list);
}

/*
 * Returns a zone suitable for an allocation with order `order` and type `type`.
 */
zone_t *zone_get(frame_order_t order, int type)
{
	list_head_t **list;
	list_head_t *l;
	zone_t *z;
	size_t i;

	if(!(list = get_list(type)))
		return NULL;
	l = *list;
	while(l)
	{
		z = CONTAINER_OF(l, zone_t, list);
		debug_assert(z->type == type, "zone: invalid zone type in list");
		i = order;
		while(i <= BUDDY_MAX_ORDER && !z->free_list[i])
			++i;
		if(i <= BUDDY_MAX_ORDER)
			return z;
		l = l->next;
	}
	return NULL;
}

/*
 * Returns the zone that owns pointer `ptr` in list `l`.Returns NULL if no zone
 * owns the pointer.
 */
zone_t *zone_get_for_(list_head_t *l, void *ptr)
{
	zone_t *z;

	while(l)
	{
		z = CONTAINER_OF(l, zone_t, list);
		if(ptr >= z->begin && ptr < z->begin + z->pages * PAGE_SIZE)
			return z;
		l = l->next;
	}
	return NULL;
}

/*
 * Returns the zone that owns pointer `ptr`. Returns NULL if no zone owns the
 * pointer.
 */
zone_t *zone_get_for(void *ptr)
{
	zone_t *z;

	if((z = zone_get_for_(dma_zone, ptr)))
		return z;
	if((z = zone_get_for_(kernel_zone, ptr)))
		return z;
	if((z = zone_get_for_(user_zone, ptr)))
		return z;
	return NULL;
}
