#include <util/util.h>

/*
 * Returns the number of elements in the given list.
 */
size_t list_size(list_head_t *list)
{
	size_t n = 0;
	list_head_t *l;

	if(!list)
		return 0;
	if((l = list->prev))
	{
		while(l->prev)
		{
			++n;
			l = l->prev;
		}
	}
	while(list)
	{
		++n;
		list = list->next;
	}
	return n;
}

/*
 * Performs the function `f` for every node in list `list`.
 */
void list_foreach(list_head_t *list, void (*f)(list_head_t *))
{
	if(!f)
		return;
	while(list)
	{
		f(list);
		list = list->next;
	}
}

/*
 * Updates the links on adjacent nodes.
 */
static void link_back(list_head_t *l)
{
	if(l->next)
		l->next->prev = l;
	if(l->prev)
		l->prev->next = l;
}

/*
 * Inserts `new_node` before `node`. If the first element of the list `first` is
 * specified, the function might change it if needed.
 */
void list_insert_before(list_head_t **first, list_head_t *node,
	list_head_t *new_node)
{
	if(!sanity_check(new_node))
		return;
	if(sanity_check(first) && *first == sanity_check(node))
		*first = new_node;
	new_node->next = node;
	new_node->prev = node->prev;
	link_back(new_node);
}

/*
 * Inserts `new_node` after `node`.
 */
void list_insert_after(list_head_t *node, list_head_t *new_node)
{
	if(!sanity_check(node) || !sanity_check(new_node))
		return;
	new_node->next = node->next;
	new_node->prev = node;
	link_back(new_node);
}

/*
 * Removes `node`. If the first element of the list `first` is specified,
 * the function might change it if needed.
 */
void list_remove(list_head_t **first, list_head_t *node)
{
	if(!sanity_check(node))
		return;
	if(sanity_check(first) && *first == node)
		*first = node->next;
	if(node->prev)
		node->prev->next = node->next;
	if(node->next)
		node->next->prev = node->prev;
}

#ifdef KERNEL_DEBUG
int list_check(list_head_t *list)
{
	while(sanity_check(list))
	{
		if(sanity_check(list->prev) && sanity_check(list->prev->next) != list)
			return 0;
		if(sanity_check(list->next) && sanity_check(list->next->prev) != list)
			return 0;
		list = list->next;
	}
	return 1;
}
#endif
