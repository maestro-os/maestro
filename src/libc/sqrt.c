#include "math.h"

// TODO errno
double sqrt(double x)
{
	double s = x;
	for(unsigned n = 0; n < 5; ++n) s = (s + (x / s)) / 2;

	return s;
}
