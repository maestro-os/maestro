#include "linked_list.h"
#include "../memory/memory.h"

static list_t *alloc_node(void *content)
{
	list_t *l;
	if(!(l = (list_t *) kmalloc(sizeof(list_t)))) return NULL;

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
	if(!l || !*l) return;

	list_t *tmp = (*l)->next;
	free((*l)->content);
	free(*l);
	*l = tmp;
}

void list_popf_front(list_t **l, void (*f)(void *))
{
	if(!l || !*l || !f) return;

	list_t *tmp = (*l)->next;
	f((*l)->content);
	free(*l);
	*l = tmp;
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
	if(!l || !*l) return;
	list_t *tmp = *l, *prev = NULL;

	while(tmp->next)
	{
		prev = tmp;
		tmp = tmp->next;
	}

	free(tmp->content);
	free(tmp);
	prev->next = NULL;
}

void list_popf_back(list_t **l, void (*f)(void *))
{
	if(!l || !*l || !f) return;
	list_t *tmp = *l, *prev = NULL;

	while(tmp->next)
	{
		prev = tmp;
		tmp = tmp->next;
	}

	prev->next = NULL;
	f(tmp->content);
	free(tmp);
}

void list_del(list_t **l, size_t i)
{
	if(!l || !*l) return;
	list_t *tmp = *l, *prev = NULL;

	while(i-- > 0 && tmp)
	{
		prev = tmp;
		tmp = tmp->next;
	}

	if(!tmp) return;
	prev->next = tmp->next;
	free(tmp->content);
	free(tmp);
}

void list_delf(list_t **l, size_t i, void (*f)(void *))
{
	if(!l || !*l || !f) return;
	list_t *tmp = *l, *prev = NULL;

	while(i-- > 0 && tmp)
	{
		prev = tmp;
		tmp = tmp->next;
	}

	if(!tmp) return;
	prev->next = tmp->next;
	f(tmp->content);
	free(tmp);
}

void list_delall(list_t **l)
{
	if(!l || !*l) return;
	list_t *t = *l, *tmp;

	while(t)
	{
		tmp = t->next;
		free(tmp->content);
		free(tmp);
		t = tmp;
	}
}

void list_delallf(list_t **l, void (*f)(void *))
{
	if(!l || !*l || !f) return;
	list_t *t = *l, *tmp;

	while(t)
	{
		tmp = t->next;
		f(tmp->content);
		free(tmp);
		t = tmp;
	}
}
