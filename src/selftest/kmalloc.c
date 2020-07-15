#include <selftest/selftest.h>
#include <memory/kmalloc/kmalloc.h>

#include <libc/stdio.h>

// TODO Require alignment?

static void test0(void)
{
	void *ptr;

	if(!(ptr = kmalloc(1)))
		ASSERT(0);
	memset(ptr, -1, 1);
	kfree(ptr);
	ASSERT(1);
}

static void test1(void)
{
	void *ptr;

	if(!(ptr = kmalloc(32)))
		ASSERT(0);
	memset(ptr, -1, 32);
	kfree(ptr);
	ASSERT(1);
}

static void test2(void)
{
	void *ptr;

	if(!(ptr = kmalloc(1000)))
		ASSERT(0);
	memset(ptr, -1, 1000);
	kfree(ptr);
	ASSERT(1);
}

static void test3(void)
{
	void *ptr;

	if(!(ptr = kmalloc(4096)))
		ASSERT(0);
	memset(ptr, -1, 4096);
	kfree(ptr);
	ASSERT(1);
}

static void test4(void)
{
	size_t i;
	void *ptr;

	for(i = 0; i < 100; ++i)
	{
		if(!(ptr = kmalloc(32)))
			ASSERT(0);
		memset(ptr, -1, 32);
		kfree(ptr);
	}
	ASSERT(1);
}

static void test5(void)
{
	size_t i;
	void *ptr[100];

	for(i = 0; i < 100; ++i)
	{
		if(!(ptr[i] = kmalloc(i * 100)))
			ASSERT(0);
		memset(ptr, -1, i * 100);
	}
	for(i = 0; i < 100; ++i)
		kfree(ptr[i]);
	ASSERT(1);
}

static int test6_(size_t i)
{
	void *ptr;
	int r;

	if(!(ptr = kmalloc(i * 100)))
		return 0;
	r = 1;
	if(i > 0)
		r = test6_(i - 1);
	kfree(ptr);
	return r;
}

static void test6(void)
{
	ASSERT(test6_(100));
}

// TODO Pseudorandom alloc size and pseudorandom free order

void test_kmalloc(void)
{
	printf("%s: ", __func__);
	test0();
	test1();
	test2();
	test3();
	test4();
	test5();
	test6();
	printf("\n");
}

static void test0_(void)
{
	size_t i = 0;

	while(kmalloc(1000))
		++i;
	printf("%zu allocations\n", i);
	ASSERT(1);
}

void test_kmalloc_bulk(void)
{
	printf("%s: ", __func__);
	test0_();
	printf("\n");
}
