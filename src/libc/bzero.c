#include "string.h"

__attribute__((hot))
void bzero(void *s, size_t n)
{
	void *end = s + n;

	while(s < n && (s & (sizeof(long) - 1) != 0))
		*(((char *) s)++) = 0;
	while(s < (end & ~((intptr_t) 7)) && (s & (sizeof(long) - 1) == 0))
		*(((long *) s)++) = 0;
	while(s < n)
		*(((char *) s)++) = 0;
}
