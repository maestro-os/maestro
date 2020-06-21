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
	if(!sanity_check(tree))
		return 0;
	return (tree->right ? tree->right->height : 0)
		- (tree->left ? tree->left->height : 0);
}

/*
 * Updates the height of all nodes in the given tree.
 */
static unsigned update_all_heights(avl_tree_t *n)
{
	unsigned left_height, right_height;

	debug_assert(n, "update_all_heights: bad argument");
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

	if(!sanity_check(root))
		return NULL;
	if(!sanity_check(new_root = root->right))
		return root;
	tmp = new_root->left;
	new_root->left = root;
	new_root->left->parent = new_root;
	if((new_root->left->right = tmp))
		new_root->left->right->parent = new_root->left;
	update_all_heights(new_root);
	return new_root;
}

/*
 * Performs a right rotation on the given root node.
 */
avl_tree_t *avl_tree_rotate_right(avl_tree_t *root)
{
	avl_tree_t *new_root, *tmp;

	if(!sanity_check(root))
		return NULL;
	if(!sanity_check(new_root = root->left))
		return root;
	tmp = new_root->right;
	new_root->right = root;
	new_root->right->parent = new_root;
	if((new_root->right->left = tmp))
		new_root->right->left->parent = new_root->right;
	update_all_heights(new_root);
	return new_root;
}

// TODO Avoid using other functions to avoid triple call to update_all_heights
avl_tree_t *avl_tree_rotate_leftright(avl_tree_t *root)
{
	avl_tree_t *new_root;

	if(!sanity_check(root))
		return NULL;
	if(!sanity_check(new_root = avl_tree_rotate_left(root->left)))
		return root;
	root->left = new_root;
	root->left->parent = root;
	return avl_tree_rotate_right(root);
}

// TODO Avoid using other functions to avoid triple call to update_all_heights
avl_tree_t *avl_tree_rotate_rightleft(avl_tree_t *root)
{
	avl_tree_t *new_root;

	if(!sanity_check(root))
		return NULL;
	if(!sanity_check(new_root = avl_tree_rotate_right(root->right)))
		return root;
	root->right = new_root;
	root->right->parent = root;
	return avl_tree_rotate_left(root);
}

/*
 * Searches a node in the given `tree` using the given `value` and comparison
 * function `f`.
 */
avl_tree_t *avl_tree_search(avl_tree_t *tree,
	const avl_value_t value, const cmp_func_t f)
{
	avl_tree_t *n;

	if(!sanity_check(tree) || !sanity_check(f))
		return NULL;
	n = tree;
	while(n->value != value)
	{
		sanity_check(n->left);
		sanity_check(n->right);
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

	while(sanity_check(n))
	{
		sanity_check(n->left);
		sanity_check(n->right);
		left_height = (n->left ? n->left->height + 1 : 0);
		right_height = (n->right ? n->right->height + 1 : 0);
		n->height = MAX(left_height, right_height);
		n = n->parent;
	}
}

/*
 * Balances the given tree after insertion of an element.
 */
static void insert_balance(avl_tree_t **tree, avl_tree_t *node)
{
	avl_tree_t *n, *g, *r;

	debug_assert(node, "insert_balance: bad arguments");
	update_heights(node);
	for(n = node->parent; n; n = n->parent)
	{
		g = n->parent;
		if(node == n->right)
		{
			if(avl_tree_balance_factor(n) > 0)
			{
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

	if(!sanity_check(tree) || !sanity_check(node) || !sanity_check(f))
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
 * Returns the leftmost node from `node`.
 */
static avl_tree_t *find_min(avl_tree_t *node)
{
	debug_assert(node, "find_min: bad argument");
	while(node->left)
		node = node->left;
	return node;
}

/*
 * Balances the given tree after removal of an element.
 */
static void remove_balance(avl_tree_t **tree, avl_tree_t *node)
{
	avl_tree_t *n, *g, *r, *tmp;
	int factor;

	debug_assert(tree && node, "remove_balance: bad arguments");
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

static void avl_tree_remove_(avl_tree_t **tree, avl_tree_t *n)
{
	avl_tree_t *tmp, *right;
	int single_node;

	debug_assert(sanity_check(tree) && sanity_check(n),
		"avl_tree_remove_: bad arguments");
	tmp = find_min(sanity_check(n->right));
	debug_assert(sanity_check(tmp), "avl_tree_remove_: bad min node");
	single_node = (sanity_check(sanity_check(tmp)->parent) == n);
	if(single_node)
	{
		tmp->left = n->left;
		tmp->left->parent = tmp;
	}
	else
	{
		if(sanity_check(right = tmp->right))
			right->parent = tmp->parent;
		tmp->parent->left = right;
	}
	if(sanity_check(n->parent))
	{
		if(n->parent->left == n)
			n->parent->left = tmp;
		else
			n->parent->right = tmp;
		tmp->parent = n->parent;
	}
	else
	{
		*tree = tmp;
		tmp->parent = NULL;
	}
	if(single_node)
		remove_balance(tree, tmp);
	else
	{
		tmp->left = n->left;
		tmp->left->parent = tmp;
		tmp->right = n->right;
		tmp->right->parent = tmp;
		if(right)
			remove_balance(tree, right);
	}
}

/*
 * Deletes the given node from the given tree.
 */
void avl_tree_remove(avl_tree_t **tree, avl_tree_t *n)
{
	avl_tree_t *tmp;

	if(!sanity_check(tree) || !sanity_check(n))
		return;
	if(sanity_check(n->left) && sanity_check(n->right))
	{
		avl_tree_remove_(tree, n);
		return;
	}
	if(n->left)
		tmp = n->left;
	else if(n->right)
		tmp = n->right;
	else
		tmp = NULL;
	if(sanity_check(n->parent))
	{
		if(n->parent->left == n)
			n->parent->left = tmp;
		else
			n->parent->right = tmp;
	}
	else
		*tree = tmp;
	if(tmp)
	{
		tmp->parent = n->parent;
		remove_balance(tree, tmp);
	}
}

/*
 * Performs the function `f` for every node in tree `tree`.
 */
void avl_tree_foreach(avl_tree_t *tree, void (*f)(avl_tree_t *))
{
	if(!sanity_check(tree))
		return;
	avl_tree_foreach(tree->left, f);
	avl_tree_foreach(tree->right, f);
	f(tree);

}

#ifdef KERNEL_DEBUG
/*
 * Checks the correctness of the given tree.
 */
int avl_tree_check(avl_tree_t *tree)
{
	if(!sanity_check(tree))
		return 1;
	if(sanity_check(tree->left) && tree->left->parent != tree)
		return 0;
	if(sanity_check(tree->right) && tree->right->parent != tree)
		return 0;
	return avl_tree_check(tree->left) && avl_tree_check(tree->right);
}

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
	if(!sanity_check(tree))
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
