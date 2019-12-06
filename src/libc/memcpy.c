#include <libc/string.h>

__attribute__((hot))
void *memcpy(void *dest, const void *src, size_t n)
{
	void *begin = dest;
	void *end = begin + n;

	while(dest < end && (((intptr_t) dest & (sizeof(long) - 1)) != 0))
		*((char *) dest++) = *((char *) src++);
	while(dest < (void *) ((intptr_t) end & ~((intptr_t) 7))
		&& (((intptr_t) dest & (sizeof(long) - 1)) == 0))
	{
		*(long *) dest = *(long *) src;
		dest += sizeof(long);
		src += sizeof(long);
	}
	while(dest < end)
		*((char *) dest++) = *((char *) src++);
	return begin;
}
