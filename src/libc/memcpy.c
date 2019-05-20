#include "string.h"

__attribute__((hot))
void *memcpy(void *dest, const void *src, size_t n)
{
	void *begin = dest;
	void *end = begin + n;

	while(dest < end && (s & (sizeof(long) - 1) != 0))
		*(((char *)dest)++) = *((char *)src++);
	while(dest < (end & ~((intptr_t) 7)) && (s & (sizeof(long) - 1) == 0))
		*(((long *)dest)++) = *((long *)src++);
	while(dest < end)
		*(((char *)dest)++) = *((char *)src++);

	return begin;
}
