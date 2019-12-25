#include <util/util.h>
#include <memory/memory.h>

rb_tree_t *rb_tree_rotate_left(rb_tree_t *node)
{
	rb_tree_t *new;

	if(!node || !(new = node->right))
		return NULL;
	node->right = new->left;
	new->left = node;
	return new;
}

rb_tree_t *rb_tree_rotate_right(rb_tree_t *node)
{
	rb_tree_t *new;

	if(!node || !(new = node->left))
		return NULL;
	node->left = new->right;
	new->right = node;
	return new;
}

rb_tree_t *rb_tree_search(rb_tree_t *tree, const uintmax_t value)
{
	while(tree && (tree->left || tree->right))
		tree = (value < tree->value ? tree->left : tree->right);
	return tree;
}

void rb_tree_insert(rb_tree_t **tree, const uintmax_t value)
{
	if(!tree)
		return;
	// TODO
	(void) value;
}

void rb_tree_delete(rb_tree_t **tree, const uintmax_t value)
{
	if(!tree)
		return;
	// TODO
	(void) value;
}

void rb_tree_freeall(rb_tree_t *tree, void (*f)(uintmax_t))
{
	if(!tree)
		return;
	rb_tree_freeall(tree->left, f);
	rb_tree_freeall(tree->right, f);
	if(f)
		f(tree->value);
	kfree(tree, 0); // TODO Slab allocation
}
