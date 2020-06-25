#include <selftest/selftest.h>
#include <memory/buddy/buddy.h>

#include <libc/stdio.h>
#include <libc/string.h>

static void test0(void)
{
	void *p;

	for(size_t i = 0; i < 1024; ++i)
	{
		if(!(p = buddy_alloc(0)))
			ASSERT(0);
		memset(p, 0xff, BLOCK_SIZE(0));
		buddy_free(p, 0);
	}
	ASSERT(1);
}

static void test1(void)
{
	void *p;

	for(size_t i = 0; i < 1024; ++i)
	{
		if(!(p = buddy_alloc(8)))
			ASSERT(0);
		memset(p, 0xff, BLOCK_SIZE(8));
		buddy_free(p, 8);
	}
	ASSERT(1);
}

static void test2(void)
{
	void *p0, *p1;

	for(size_t i = 0; i < 1024; ++i)
	{
		if(!(p0 = buddy_alloc(8)))
			ASSERT(0);
		memset(p0, 0xff, BLOCK_SIZE(8));
		if(!(p1 = buddy_alloc(0)))
		{
			buddy_free(p0, 8);
			ASSERT(0);
		}
		memset(p1, 0xff, BLOCK_SIZE(0));
		buddy_free(p1, 0);
		buddy_free(p0, 8);
	}
	ASSERT(1);
}

/*static void test3(void)
{
	void *p;

	for(size_t i = 0; i < 1024; ++i)
	{
		if(!(p = buddy_alloc(8)))
			ASSERT(0);
		memset(p, 0xff, BLOCK_SIZE(8));
	}
	ASSERT(1);
}*/

void test_buddy(void)
{
	printf("%s: ", __func__);
	test0();
	test1();
	test2();
	//test3();
	printf("\n");
}

typedef struct buddy_block_test
{
	struct buddy_block_test *next;
	size_t order;
} buddy_block_test_t;

static int check_duplicates(const buddy_block_test_t *blocks,
	const buddy_block_test_t *b)
{
	while(blocks)
	{
		if(blocks == b)
			return 1;
		blocks = blocks->next;
	}
	return 0;
}

static int test0__(const block_order_t order)
{
	buddy_block_test_t *blocks = NULL, *b, *next;
	size_t i = 0;

	while((b = buddy_alloc(order)))
	{
		if(check_duplicates(blocks, b))
		{
			printf("DUPLICATE: %p\n", b);
			return 0;
		}
		b->next = blocks;
		b->order = order;
		blocks = b;
		if(++i % 16384 == 0)
			printf("%zu allocations of order %u\n", i, order);
	}
	printf("%zu allocations of order %u\n", i, order);
	i = 0;
	while(blocks)
	{
		next = blocks->next;
		buddy_free(blocks, blocks->order);
		blocks = next;
		if(++i % 16384 == 0)
			printf("%zu free\n", i);
	}
	printf("%zu free\n", i);
	return 1;
}

static void test0_(void)
{
	const size_t max = 10;
	size_t i;

	i = max;
	while(1)
	{
		printf("Buddy duplicate testing: %zu/%zu\n", i, max);
		if(!test0__(i))
			ASSERT(0);
		if(i == 0)
			break;
		--i;
	}
	ASSERT(1);
}

void test_buddy_duplicates(void)
{
	printf("%s: ", __func__);
	test0_();
	printf("\n");
}
