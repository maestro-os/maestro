#include "linked_list.h"

static list_t *alloc_node(void *content)
{
	list_t *l;
	if(!(l = (list_t *) malloc(sizeof(list_t)))) return NULL;

	l->content = content;
	l->next = NULL;

	return l;
}

size_t list_size(const list_t *l)
{
	size_t i = 0;

	while(l)
	{
		++i;
		l = l->next;
	}

	return i;
}

list_t *list_get(list_t *l, size_t i)
{
	size_t j = 0;

	while(j < i && l)
	{
		++j;
		l = l->next;
	}

	return l;
}

list_t *list_back(list_t *l)
{
	if(!l) return NULL;
	while(l->next) l = l->next;

	return l;
}

void list_foreach(const list_t *l, void (*f)(void *))
{
	while(l)
	{
		f(l->content);
		l = l->next;
	}
}

void list_set(list_t **l, size_t i, void *content)
{
	if(!l) return;

	if(!(*l))
	{
		*l = alloc_node(content);
		return;
	}

	size_t j = 0;
	list_t *n = *l;

	while(j < i && n)
	{
		++i;

		if(!n->next)
		{
			n->next = alloc_node(content);
			return;
		}

		n = n->next;
	}

	if(n) n->content = content;
}

void list_push_front(list_t **l, void *content)
{
	if(!l) return;

	list_t *n = *l;
	*l = alloc_node(content);
	(*l)->next = n;
}

void list_pop_front(list_t **l)
{
	(void) l;
	// TODO
}

void list_popf_front(list_t **l, void (*f)(void *))
{
	(void) l;
	(void) f;
	// TODO
}

void list_push_back(list_t **l, void *content)
{
	if(!l) return;

	if(!(*l))
	{
		*l = alloc_node(content);
		return;
	}

	list_t *n = *l;
	while(n->next) n = n->next;

	n->next = alloc_node(content);
}

void list_pop_back(list_t **l)
{
	(void) l;
	// TODO
}

void list_popf_back(list_t **l, void (*f)(void *))
{
	(void) l;
	(void) f;
	// TODO
}

void list_delete(list_t **l, size_t i)
{
	(void) l;
	(void) i;
	// TODO
}

void list_deletef(list_t **l, size_t i, void (*f)(void *))
{
	(void) l;
	(void) i;
	(void) f;
	// TODO
}

void list_deleteall(list_t **l)
{
	(void) l;
	// TODO
}

void list_deleteallf(list_t **l, void (*f)(void *))
{
	(void) l;
	(void) f;
	// TODO
}
