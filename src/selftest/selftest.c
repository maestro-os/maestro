#include <selftest/selftest.h>
#include <libc/string.h>

static test_suite_func_t suites[] = {
	test_bitfield,
	test_avl,
	//test_buddy,
	test_pages
};

void run_selftest(void)
{
	size_t i;

	for(i = 0; i < sizeof(suites) / sizeof(test_suite_func_t); ++i)
		suites[i]();
}
