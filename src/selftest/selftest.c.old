#include <selftest/selftest.h>
#include <libc/string.h>

static test_suite_func_t suites[] = {
	//test_bitfield,
	//test_avl,
	test_buddy,
	//test_kmalloc,

	//test_buddy_duplicates,
	//test_kmalloc_bulk
};

void run_selftest(void)
{
	size_t i;

	for(i = 0; i < sizeof(suites) / sizeof(test_suite_func_t); ++i)
		suites[i]();
}
