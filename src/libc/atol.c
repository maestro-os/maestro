#include "stdlib.h"

long atol(const char *nptr)
{
	while(*nptr && *nptr <= ' ') ++nptr;

	const int neg = (*nptr == '-');
	if(neg || *nptr == '+') ++nptr;

	long n = 0;

	while(*nptr >= '0' && *nptr <= '9')
	{
		n *= 10;
		n += *(nptr++) - '0';
	}

	return (neg ? -n : n);
}
