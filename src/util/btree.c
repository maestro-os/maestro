#include "btree.h"
#include "../memory/memory.h"

btree_t *btree_new(void *content)
{
	btree_t *t;

	if(!(t = kmalloc(sizeof(btree_t)))) return NULL;
	bzero(t, sizeof(btree_t));

	t->content = content;
	return t;
}

void btree_foreach(btree_t *t, void (*f)(void *))
{
	if(!f || !f) return;

	btree_foreach(t->left, f);
	btree_foreach(t->right, f);
}

btree_t *btree_search(btree_t *t, int (*cmp)(void *, void *), void *needle)
{
	if(!t || !cmp) return NULL;
	const int i = cmp(t->content, needle);

	if(i == 0)
		return t;
	else if(i < 0)
		return btree_search(t->left, cmp, needle);
	else
		return btree_search(t->right, cmp, needle);
}

void btree_del(btree_t *t, btree_t **left, btree_t **right)
{
	if(left) *left = t->left;
	if(right) *right = t->right;

	kfree(t->content);
	kfree(t);
}

void btree_delf(btree_t *t, btree_t **left,
	btree_t **right, void (*f)(void *))
{
	if(left) *left = t->left;
	if(right) *right = t->right;

	f(t->content);
	kfree(t);
}

void btree_delall(btree_t **t)
{
	if(!t) return;

	btree_t *left, *right;
	btree_del(*t, &left, &right);

	btree_delall(&left);
	btree_delall(&right);
	*t = NULL;
}

void btree_delallf(btree_t **t, void (*f)(void *))
{
	if(!t || !f) return;

	btree_t *left, *right;
	btree_delf(*t, &left, &right, f);

	btree_delallf(&left, f);
	btree_delallf(&right, f);
	*t = NULL;
}
