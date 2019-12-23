#include <util/util.h>
#include <memory/memory.h>

void rb_tree_freeall(rb_tree_t **tree)
{
	if(!tree)
		return;
	if(!((*tree)->flags & RB_TREE_FLAG_LEFT_LEAF))
		rb_tree_freeall(&(*tree)->left);
	if(!((*tree)->flags & RB_TREE_FLAG_RIGHT_LEAF))
		rb_tree_freeall(&(*tree)->right);
	kfree(*tree, 0); // TODO Slab allocation
}
