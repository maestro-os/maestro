#include <util/util.h>

unsigned floor_log2(const unsigned n)
{
	unsigned r = 0;
	while(POW2(r) < n) ++r;
	if(POW2(r) > n) --r;

	return r;
}
