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
		memset(p, 0xff, PAGE_SIZE << 8);
		buddy_free(p, 0);
	}
}

static void test1(void)
{
	void *p;

	for(size_t i = 0; i < 1024; ++i)
	{
		if(!(p = buddy_alloc(8)))
			ASSERT(0);
		memset(p, 0xff, PAGE_SIZE << 8);
		buddy_free(p, 0);
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
		memset(p0, 0xff, PAGE_SIZE << 8);
		if(!(p1 = buddy_alloc(0)))
		{
			buddy_free(p0, 8);
			ASSERT(0);
		}
		memset(p1, 0xff, PAGE_SIZE << 8);
		buddy_free(p1, 0);
		buddy_free(p0, 8);
	}
	ASSERT(1);
}

static void test3(void)
{
	void *p;

	for(size_t i = 0; i < 1024; ++i)
	{
		if(!(p = buddy_alloc(8)))
			ASSERT(0);
		memset(p, 0xff, PAGE_SIZE << 8);
	}
	ASSERT(1);
}

void test_buddy(void)
{
	printf("%s: ", __func__);
	test0();
	test1();
	test2();
	test3();
	printf("\n");
}
