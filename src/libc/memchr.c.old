#include <libc/string.h>

void *memchr(const void *s, int c, size_t n)
{
	while(n--)
	{
		if(*((char *) s) == c)
			return (void *) s;
		++s;
	}
	return NULL;
}
