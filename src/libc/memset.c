#include "string.h"

__attribute__((hot))
void *memset(void *s, int c, size_t n)
{
	void *begin = s;
	void *end = begin + n;

	const long field = make_field(c);

	while(s < end && (s & (sizeof(long) - 1) != 0))
		*(((char *) s)++) = c;
	while(s < (end & ~((intptr_t) 7)) && (s & (sizeof(long) - 1) == 0))
		*(((long *) s)++) = val;
	while(s < end)
		*(((char *) s)++) = c;

	return begin;
}
