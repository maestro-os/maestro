#include <util/util.h>

/*
 * Computes floor(log2(n)) on the given unsigned integer `n` without using
 * floating point numbers.
 */
unsigned floor_log2(const unsigned n)
{
	unsigned r = 0;

	while(POW2(r) < n)
		++r;
	if(POW2(r) > n)
		--r;
	return r;
}
