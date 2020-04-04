#include <selftest/selftest.h>
#include <memory/pages/pages.h>

#include <libc/stdio.h>
#include <libc/string.h>

static void test0(void)
{
	void *p;

	for(size_t i = 0; i < 1024; ++i)
	{
		if(!(p = pages_alloc(1)))
			ASSERT(0);
		memset(p, 0xff, PAGE_SIZE);
		pages_free(p, 1);
	}
	ASSERT(1);
}

static void test1(void)
{
	void *p;

	for(size_t i = 0; i < 1024; ++i)
	{
		if(!(p = pages_alloc(2)))
			ASSERT(0);
		memset(p, 0xff, PAGE_SIZE * 2);
		pages_free(p, 2);
	}
	ASSERT(1);
}

static void test2(void)
{
	void *p;

	for(size_t i = 0; i < 1024; ++i)
	{
		if(!(p = pages_alloc(9)))
			ASSERT(0);
		memset(p, 0xff, PAGE_SIZE * 9);
		pages_free(p, 9);
	}
	ASSERT(1);
}

static void test3(void)
{
	void *p0, *p1;

	for(size_t i = 0; i < 1024; ++i)
	{
		if(!(p0 = pages_alloc(9)))
			ASSERT(0);
		memset(p0, 0xff, PAGE_SIZE * 9);
		if(!(p1 = pages_alloc(1)))
		{
			buddy_free(p0, 9);
			ASSERT(0);
		}
		memset(p1, 0xff, PAGE_SIZE);
		pages_free(p1, 1);
		pages_free(p0, 9);
	}
	ASSERT(1);
}

/*static void test4(void)
{
	void *p;

	while(1)
	{
		if(!(p = pages_alloc(128)))
			break;
		memset(p, 0xff, PAGE_SIZE * 128);
	}
	ASSERT(1);
}*/

void test_pages(void)
{
	printf("%s: ", __func__);
	test0();
	test1();
	test2();
	test3();
	//test4();
	printf("\n");
}
