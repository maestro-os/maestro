#include <stdint.h>
#include <stddef.h>

#include "libc.h"

// TODO Optimize
void *memmove(void *dest, const void *src, size_t n)
{
	void *begin = dest;
	size_t i = 0;

	if (dest < src)
		return memcpy(dest, src, n);
	while (i < n)
	{
		((char *) dest)[n - i - 1] = ((char *) src)[n - i - 1];
		++i;
	}
	return begin;
}
