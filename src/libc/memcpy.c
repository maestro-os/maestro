#include "string.h"

// TODO Rewrite
void *memcpy(void *dest, const void *src, size_t n)
{
	if((uintptr_t) dest % sizeof(long) == 0
		&& (uintptr_t) src % sizeof(long) == 0
		&& n % sizeof(long) == 0)
	{
		for(size_t i = 0; i < n / sizeof(long); ++i)
			*((long *) dest + i) = *((long *) src + i);
	}
	else
	{
		for(size_t i = 0; i < n; ++i)
			*((char *) dest + i) = *((char *) src + i);
	}

	return dest;
}
