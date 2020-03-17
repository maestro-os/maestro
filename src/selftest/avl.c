#include <selftest/selftest.h>
#include <util/util.h>

#include <libc/stdio.h>

static void test0(void)
{
	avl_tree_t a;
	avl_tree_t b;

	bzero(&a, sizeof(a));
	bzero(&b, sizeof(b));
	a.right = &b;
	b.parent = &a;
	avl_tree_rotate_left(&a);
	if(a.parent != &b)
		ASSERT(0);
	if(a.left != NULL)
		ASSERT(0);
	if(a.right != NULL)
		ASSERT(0);
	if(b.left != &a)
		ASSERT(0);
	if(b.right != NULL)
		ASSERT(0);
	ASSERT(1);
}

static void test1(void)
{
	avl_tree_t a;
	avl_tree_t b;

	bzero(&a, sizeof(a));
	bzero(&b, sizeof(b));
	a.left = &b;
	b.parent = &a;
	avl_tree_rotate_right(&a);
	if(a.parent != &b)
		ASSERT(0);
	if(a.left != NULL)
		ASSERT(0);
	if(a.right != NULL)
		ASSERT(0);
	if(b.left != NULL)
		ASSERT(0);
	if(b.right != &a)
		ASSERT(0);
	ASSERT(1);
}

void test_avl(void)
{
	printf("%s: ", __func__);
	test0();
	test1();
	printf("\n");
}
