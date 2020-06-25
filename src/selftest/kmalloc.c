#include <selftest/selftest.h>
#include <memory/kmalloc/kmalloc.h>

#include <libc/stdio.h>

static void test0(void)
{
	while(kmalloc(1000))
		;
	ASSERT(1);
}

void test_kmalloc_bulk(void)
{
	printf("%s: ", __func__);
	test0();
	printf("\n");
}
