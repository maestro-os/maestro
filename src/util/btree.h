#ifndef BTREE_H
# define BTREE_H

# include "../libc/stdlib.h"

typedef struct btree
{
	void *content;
	struct btree *left;
	struct btree *right;
} btree_t;

btree_t *btree_new(void *content);
void btree_foreach(btree_t *t, void (*f)(void *));
btree_t *btree_search(btree_t *t, int (*cmp)(void *, void *), void *needle);
void btree_del(btree_t *t, btree_t **left, btree_t **right);
void btree_delf(btree_t *t, btree_t **left,
	btree_t **right, void (*f)(void *));
void btree_delall(btree_t **t);
void btree_delallf(btree_t **t, void (*f)(void *));

#endif
