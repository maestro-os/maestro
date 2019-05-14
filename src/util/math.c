#include "util.h"

unsigned ulog2(const unsigned n)
{
	unsigned r = 0;
	while((unsigned) POW2(r + 1) < n) ++r;

	return r;
}
