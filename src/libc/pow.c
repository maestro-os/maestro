#include <libc/math.h>
#include <libc/string.h>

// TODO Add not-round and negative y handling
double pow(double x, double y)
{
	double n = x;
	for(size_t i = 0; i < y; ++i) n *= x;

	return n;
}
