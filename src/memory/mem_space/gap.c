#include <kernel.h>
#include <memory/mem_space/mem_space.h>
#include <memory/slab/slab.h>

/*
 * This file handles gaps for memory spaces.
 *
 * A gap represent a zone of the virtual memory that is available for
 * allocation. When a region is allocated, it must take the place of a gap and
 * when a region is freed, a gap is created at its place.
 */

/*
 * The cache for the `mem_gap` structure.
 */
static cache_t *mem_gap_cache;

/*
 * Initializes gaps.
 */
static void gaps_global_init(void)
{
	mem_gap_cache = cache_create("mem_gap", sizeof(mem_gap_t), 64,
		bzero, NULL);
	if(!mem_gap_cache)
		PANIC("Memory spaces initialization failed!", 0);
}

/*
 * Creates a memory gap with the given values for the given memory space. The
 * gap is not inserted in any data structure.
 */
mem_gap_t *gap_create(mem_space_t *space, void *begin, const size_t pages)
{
	static int init = 0;
	mem_gap_t *gap;

	if(unlikely(!init))
	{
		gaps_global_init();
		init = 1;
	}
	debug_assert(sanity_check(space), "Invalid memory space");
	ASSERT_RANGE(begin, pages);
	if(!(gap = cache_alloc(mem_gap_cache)))
		return NULL;
	gap->begin = begin;
	gap->pages = pages;
	gap->mem_space = space;
	gap->node.value = pages;
	return gap;
}

/*
 * Clones the given gap for the given destination space. The gap is not inserted
 * in any data structure.
 */
mem_gap_t *gap_clone(mem_space_t *dest, mem_gap_t *g)
{
	mem_gap_t *new;

	if(!sanity_check(new = gap_create(dest, g->begin, g->pages)))
		return NULL;
	return new;
}

/*
 * Clones the given gaps list to the given destination memory space.
 *
 * On success, returns 1. On fail, returns 0.
 */
int gaps_clone(mem_space_t *dest, list_head_t *gaps)
{
	list_head_t *l;
	mem_gap_t *g, *new;
	list_head_t *last = NULL;

	for(l = gaps; l; l = l->next)
	{
		g = CONTAINER_OF(l, mem_gap_t, list);
		if(!sanity_check(new = gap_clone(dest, g)))
		{
			gaps_free(dest->gaps);
			dest->gaps = NULL;
			return 0;
		}
		list_insert_after(&dest->gaps, last, &new->list);
		avl_tree_insert(&dest->free_tree, &new->node, avl_val_cmp);
		last = &new->list;
	}
	return 1;
}

/*
 * Finds a gap large enough to fit the required number of pages.
 * If no gap large enough is found, `NULL` is returned.
 */
avl_tree_t *gap_find(avl_tree_t *n, const size_t pages)
{
	if(!sanity_check(n) || pages == 0)
		return NULL;
	while(1)
	{
		if(n->left
			&& CONTAINER_OF(n->left, mem_gap_t, node)->pages > pages)
			n = n->left;
		else if(n->right
			&& CONTAINER_OF(n->right, mem_gap_t, node)->pages < pages)
			n = n->right;
		else
			break;
	}
	return n;
}

/*
 * Creates the default memory gaps for the given memory space.
 */
int gaps_init(mem_space_t *s)
{
	void *gap_begin;
	size_t gap_pages;
	mem_gap_t *gap;

	debug_assert(sanity_check(s), "Invalid memory space");
	gap_begin = MEM_SPACE_BEGIN;
	gap_pages = (uintptr_t) KERNEL_BEGIN / PAGE_SIZE - 1;
	if(!(gap = gap_create(s, gap_begin, gap_pages)))
		return 0;
	list_insert_after(&s->gaps, s->gaps, &gap->list);
	avl_tree_insert(&s->free_tree, &gap->node, avl_val_cmp);
	// TODO Only expose a stub for the kernel
	gap_begin = mem_info.heap_begin;
	gap_pages = (MEM_SPACE_END - mem_info.heap_begin) / PAGE_SIZE;
	if(!(gap = gap_create(s, gap_begin, gap_pages)))
		return 0;
	list_insert_after(&s->gaps, s->gaps, &gap->list);
	avl_tree_insert(&s->free_tree, &gap->node, avl_val_cmp);
	return 1;
}

/*
 * Creates a gap of the specified size at the specified address and/or
 * extend/merge gaps around.
 */
int gap_extend(avl_tree_t **tree, void *addr, const size_t pages)
{
	debug_assert(sanity_check(tree), "Invalid gaps tree");
	ASSERT_RANGE(addr, pages);
	// TODO
	return 1;
}

/*
 * Shrinks the given gap to the given amount of pages. The pointer to the
 * beginning of the gap will increase to that amount of pages and the location
 * of the gap in its tree will be updated.
 */
void gap_shrink(avl_tree_t **tree, avl_tree_t *gap, const size_t pages)
{
	mem_gap_t *g;

	debug_assert(sanity_check(tree), "Invalid gaps tree");
	if(!gap || pages == 0)
		return;
	g = CONTAINER_OF(gap, mem_gap_t, node);
	debug_assert(pages <= g->pages, "Gap is too small");
	g->begin += pages * PAGE_SIZE;
	g->pages -= pages;
	if(g->pages <= 0)
	{
		gap_free(g);
		return;
	}
	avl_tree_remove(tree, gap);
	g->node.value = g->pages;
	avl_tree_insert(tree, gap, ptr_cmp);
}

/*
 * Unlinks and frees the given gap.
 */
void gap_free(mem_gap_t *gap)
{
	mem_space_t *mem_space;

	debug_assert(sanity_check(gap), "Invalid gap");
	mem_space = gap->mem_space;
	debug_assert(sanity_check(mem_space), "Invalid memory space");
	list_remove(&mem_space->gaps, &gap->list);
	avl_tree_remove(&mem_space->free_tree, &gap->node);
	cache_free(mem_gap_cache, gap);
}

/*
 * Removes the gap structure in the given list node.
 */
static void list_free_gap(list_head_t *l)
{
	debug_assert(sanity_check(l), "Invalid list");
	gap_free(CONTAINER_OF(l, mem_gap_t, list));
}

/*
 * Removes all gaps in the given list.
 * The pointer to the list has to be considered invalid after calling this
 * funciton.
 */
void gaps_free(list_head_t *list)
{
	debug_assert(sanity_check(list), "Invalid list");
	list_foreach(list, list_free_gap);
}
