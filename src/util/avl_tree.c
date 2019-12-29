#include <util/util.h>
#include <memory/memory.h>
#include <kernel.h>

static cache_t *avl_tree_cache;

static void global_init(void)
{
	if(!(avl_tree_cache = cache_create("avl_tree", sizeof(avl_tree_t), 256,
		bzero, NULL)))
		PANIC("Failed to initialize avl_tree cache!", 0);
}

static avl_tree_t *create_node(void *value)
{
	static int init = 0;
	avl_tree_t *node;

	if(!init)
	{
		global_init();
		init = 1;
	}
	if((node = cache_alloc(avl_tree_cache)))
		node->value = value;
	return node;
}

int avl_tree_balance_factor(const avl_tree_t *tree)
{
	return (tree->right ? tree->right->height : 0)
		- (tree->left ? tree->left->height : 0);
}

static unsigned update_all_heights(avl_tree_t *n)
{
	unsigned left_height, right_height;

	left_height = (n->left ? update_all_heights(n->left) + 1 : 0);
	right_height = (n->right ? update_all_heights(n->right) + 1 : 0);
	return n->height = MAX(left_height, right_height);
}

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

avl_tree_t *avl_tree_search(avl_tree_t *tree, void *value, const cmp_func_t f)
{
	avl_tree_t *n;

	if(!tree || !f)
		return NULL;
	n = tree;
	while(n->value != value)
	{
		if(f(n->value, value) < 0 && n->left)
			n = n->left;
		else if(n->right)
			n = n->right;
		else
			return NULL;
	}
	return n;
}

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

void avl_tree_insert(avl_tree_t **tree, void *value, const cmp_func_t f)
{
	avl_tree_t *node, *n;
	int i = 0;

	if(!tree || !(node = create_node(value)))
		return;
	if((n = *tree))
	{
		while(1)
		{
			i = f(n->value, value);
			if(i < 0 && n->left)
				n = n->left;
			else if(i > 0 && n->right)
				n = n->right;
			else
				break;
		}
		if(i == 0)
		{
			cache_free(avl_tree_cache, node);
			return;
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

static avl_tree_t *find_min(avl_tree_t *node)
{
	while(node->left)
		node = node->left;
	return node;
}

void avl_tree_delete(avl_tree_t **tree, avl_tree_t *n)
{
	avl_tree_t *tmp;

	if(!tree || !n)
		return;
	if(n->left && n->right)
	{
		tmp = find_min(n);
		n->value = tmp->value;
		avl_tree_delete(tree, tmp);
	}
	else
	{
		if(n->left)
			tmp = n->left;
		else if(n->right)
			tmp = n->right;
		else
			tmp = NULL;
		if(n == n->parent->left)
			n->parent->left = tmp;
		else
			n->parent->right = tmp;
		// TODO Rebalance
	}
	cache_free(avl_tree_cache, n);
}

void avl_tree_freeall(avl_tree_t *tree, void (*f)(void *))
{
	if(!tree)
		return;
	if(f)
		f(tree->value);
	avl_tree_freeall(tree->left, f);
	avl_tree_freeall(tree->right, f);
	cache_free(avl_tree_cache, tree);
}

#ifdef KERNEL_DEBUG
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

void avl_tree_print(const avl_tree_t *tree)
{
	avl_tree_print_(tree, 0);
}
#endif
