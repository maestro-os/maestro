#ifndef LINKED_LIST_H
# define LINKED_LIST_H

# include "../libc/stdlib.h"

typedef struct list
{
	void *content;
	struct list *next;
} list_t;

size_t list_size(const list_t *l);
list_t *list_get(list_t *l, size_t i);
list_t *list_back(list_t *l);
void list_foreach(const list_t *l, void (*f)(void *));
void list_set(list_t **l, size_t i, void *content);
void list_push_front(list_t **l, void *content);
void list_pop_front(list_t **l);
void list_popf_front(list_t **l, void (*f)(void *));
void list_push_back(list_t **l, void *content);
void list_pop_back(list_t **l);
void list_popf_back(list_t **l, void (*f)(void *));
void list_del(list_t **l, size_t i);
void list_delf(list_t **l, size_t i, void (*f)(void *));
void list_delall(list_t **l);
void list_delallf(list_t **l, void (*f)(void *));

#endif
