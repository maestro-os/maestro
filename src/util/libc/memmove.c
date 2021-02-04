#include <stdint.h>
#include <stddef.h>

void *memcpy(void *dest, const void *src, size_t n);

// TODO Optimize
/*
 * Same as memcpy, except the function can handle overlapping memory areas.
 */
void *memmove(void *dest, const void *src, size_t n)
{
	void *begin = dest;
	size_t i = 0;

	if(dest < src)
		return memcpy(dest, src, n);

	while(i < n)
	{
		((char *) dest)[n - i - 1] = ((char *) src)[n - i - 1];
		++i;
	}
	return begin;
}
