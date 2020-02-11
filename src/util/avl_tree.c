#include <util/util.h>
#include <memory/memory.h>
#include <kernel.h>

/*
 * This file contains functions for AVL trees handling.
 */

/*
 * Returns the balance factor for the given tree.
 */
int avl_tree_balance_factor(const avl_tree_t *tree)
{
	return (tree->right ? tree->right->height : 0)
		- (tree->left ? tree->left->height : 0);
}

/*
 * Updates the height of all nodes in the given tree.
 */
static unsigned update_all_heights(avl_tree_t *n)
{
	unsigned left_height, right_height;

	left_height = (n->left ? update_all_heights(n->left) + 1 : 0);
	right_height = (n->right ? update_all_heights(n->right) + 1 : 0);
	return n->height = MAX(left_height, right_height);
}

/*
 * Performs a left rotation on the given root node.
 */
avl_tree_t *avl_tree_rotate_left(avl_tree_t *root)
{
	avl_tree_t *new_root, *tmp;

	if(!root || !(new_root = root->right))
		return NULL;
	tmp = new_root->left;
	new_root->left = root;
	new_root->left->parent = new_root;
	if((root->right = tmp))
		root->right->parent = root;
	update_all_heights(new_root);
	return new_root;
}

/*
 * Performs a right rotation on the given root node.
 */
avl_tree_t *avl_tree_rotate_right(avl_tree_t *root)
{
	avl_tree_t *new_root, *tmp;

	if(!root || !(new_root = root->left))
		return NULL;
	tmp = new_root->right;
	new_root->right = root;
	new_root->right->parent = new_root;
	if((root->left = tmp))
		root->left->parent = root;
	update_all_heights(new_root);
	return new_root;
}

// TODO Avoid using other functions to avoid triple call to update_all_heights
avl_tree_t *avl_tree_rotate_leftright(avl_tree_t *root)
{
	avl_tree_t *new_root;

	if(!root || !(new_root = avl_tree_rotate_left(root->right)))
		return NULL;
	root->right = new_root;
	root->right->parent = root;
	return avl_tree_rotate_right(root);
}

// TODO Avoid using other functions to avoid triple call to update_all_heights
avl_tree_t *avl_tree_rotate_rightleft(avl_tree_t *root)
{
	avl_tree_t *new_root;

	if(!root || !(new_root = avl_tree_rotate_right(root->left)))
		return NULL;
	root->left = new_root;
	root->left->parent = root;
	return avl_tree_rotate_left(root);
}

/*
 * Searches a node in the given tree using the given value
 * and comparison function.
 */
avl_tree_t *avl_tree_search(avl_tree_t *tree,
	const avl_value_t value, const cmp_func_t f)
{
	avl_tree_t *n;

	if(!tree || !f)
		return NULL;
	n = tree;
	while(n->value != value)
	{
		if(f(&n->value, &value) < 0 && n->left)
			n = n->left;
		else if(n->right)
			n = n->right;
		else
			return NULL;
	}
	return n;
}

/*
 * Updates the height of the node `n` and its parents.
 */
static void update_heights(avl_tree_t *n)
{
	unsigned left_height, right_height;

	while(n)
	{
		if(n->left || n->right)
		{
			left_height = (n->left ? n->left->height : 0);
			right_height = (n->right ? n->right->height : 0);
			n->height = MAX(left_height, right_height) + 1;
		}
		else
			n->height = 0;
		n = n->parent;
	}
}

/*
 * Balances the given tree after insertion of an element.
 */
static void insert_balance(avl_tree_t **tree, avl_tree_t *node)
{
	avl_tree_t *n, *g, *r;

	update_heights(node);
	for(n = node->parent; n; n = n->parent)
	{
		if(node == n->right)
		{
			if(avl_tree_balance_factor(n) > 0)
			{
				g = n->parent;
				if(avl_tree_balance_factor(node) < 0)
					r = avl_tree_rotate_rightleft(n);
				else
					r = avl_tree_rotate_left(n);
			}
			else
			{
				if(avl_tree_balance_factor(n) < 0)
					break;
				node = n;
				continue;
			}
		}
		else
		{
			if(avl_tree_balance_factor(n) < 0)
			{
				g = n->parent;
				if(avl_tree_balance_factor(node) > 0)
					r = avl_tree_rotate_leftright(n);
				else
					r = avl_tree_rotate_right(n);
			}
			else
			{
				if(avl_tree_balance_factor(n) > 0)
					break;
				node = n;
				continue;
			}
		}
		if((r->parent = g))
		{
			if(n == g->left)
				g->left = r;
			else
				g->right = r;
		}
		else
			*tree = r;
		break;
	}
}

/*
 * Inserts the given `node` into the given `tree` using the given comparison
 * function `f`.
 * If the node has a value equivalent to the value of another node, it will be
 * inserted in the right subtree of that node.
 */
void avl_tree_insert(avl_tree_t **tree, avl_tree_t *node, const cmp_func_t f)
{
	avl_tree_t *n;
	int i = 0;

	if(!tree)
		return;
	node->left = NULL;
	node->right = NULL;
	node->parent = NULL;
	node->height = 0;
	if((n = *tree))
	{
		while(1)
		{
			i = f(&n->value, &node->value);
			if(i < 0 && n->left)
				n = n->left;
			else if(i > 0 && n->right)
				n = n->right;
			else
				break;
		}
		if(i < 0)
			n->left = node;
		else
			n->right = node;
		node->parent = n;
		insert_balance(tree, node);
	}
	else
		*tree = node;
}

/*
 * TODO
 */
static avl_tree_t *find_min(avl_tree_t *node)
{
	while(node->left)
		node = node->left;
	return node;
}

/*
 * Balances the given tree after deletion of an element.
 */
static void delete_balance(avl_tree_t **tree, avl_tree_t *node)
{
	avl_tree_t *n, *g, *r, *tmp;
	int factor;

	update_heights(node);
	r = node;
	for(n = r->parent; n; n = g)
	{
		g = n->parent;
		if(r == n->left)
		{
			if(avl_tree_balance_factor(n) > 0)
			{
				tmp = n->right;
				factor = avl_tree_balance_factor(tmp);
				if(factor < 0)
					r = avl_tree_rotate_rightleft(n);
				else
					r = avl_tree_rotate_left(n);
			}
			else
			{
				if(avl_tree_balance_factor(n) == 0)
					break;
				r = n;
				continue;
			}
		}
		else
		{
			if(avl_tree_balance_factor(n) < 0)
			{
				tmp = n->left;
				factor = avl_tree_balance_factor(tmp);
				if(factor > 0)
					r = avl_tree_rotate_leftright(n);
				else
					r = avl_tree_rotate_right(n);
			}
			else
			{
				if(avl_tree_balance_factor(n) == 0)
					break;
				r = n;
				continue;
			}
		}
		if((r->parent = g))
		{
			if(n == g->left)
				g->left = r;
			else
				g->right = r;
			if(factor == 0)
				break;
		}
		else
			*tree = r;
	}
}

/*
 * Deletes the given node fron the given tree.
 */
void avl_tree_remove(avl_tree_t **tree, avl_tree_t *n)
{
	avl_tree_t *tmp;

	if(!tree || !n)
		return;
	if(n->left && n->right)
	{
		tmp = find_min(n);
		n->value = tmp->value;
		avl_tree_remove(tree, tmp);
	}
	else
	{
		if(n->left)
			tmp = n->left;
		else if(n->right)
			tmp = n->right;
		else
			tmp = NULL;
		if(n->parent)
		{
			if(n == n->parent->left)
				n->parent->left = tmp;
			else
				n->parent->right = tmp;
		}
		else
			*tree = tmp;
		if(tmp) // TODO Check that this is correct (is tree still balanced?)
		{
			tmp->parent = n->parent;
			delete_balance(tree, tmp);
		}
	}
}

/*
 * Performs the function `f` for every node in tree `tree`.
 */
void avl_tree_foreach(avl_tree_t *tree, void (*f)(avl_tree_t *))
{
	if(!tree)
		return;
	avl_tree_foreach(tree->left, f);
	avl_tree_foreach(tree->right, f);
	f(tree);
}

#ifdef KERNEL_DEBUG
/*
 * Prints `n` tabs.
 */
static void print_tabs(size_t n)
{
	while(n--)
		printf("\t");
}

void avl_tree_print_(const avl_tree_t *tree, const size_t level)
{
	if(!tree)
		return;
	// TODO Use %ju?
	printf("%lu - Height: %u\n", (long unsigned)tree->value, tree->height);
	if(tree->left)
	{
		print_tabs(level + 1);
		printf("Left: ");
		avl_tree_print_(tree->left, level + 1);
	}
	if(tree->right)
	{
		print_tabs(level + 1);
		printf("Right: ");
		avl_tree_print_(tree->right, level + 1);
	}
}

/*
 * Prints the given AVL tree.
 */
void avl_tree_print(const avl_tree_t *tree)
{
	if(tree)
		avl_tree_print_(tree, 0);
	else
		printf("(Empty tree)\n");
}
#endif
