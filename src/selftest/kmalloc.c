#include <selftest/selftest.h>
#include <memory/kmalloc/kmalloc.h>

#include <libc/stdio.h>

static void test0(void)
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
	test0();
	printf("\n");
}
