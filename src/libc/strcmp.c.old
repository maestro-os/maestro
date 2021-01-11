#include <libc/string.h>

// TODO Optimize
int strcmp(const char *s1, const char *s2)
{
	if(!s1 || !s2) return 0;

	while(*s1 && *s2 && *s1 == *s2)
	{
		++s1;
		++s2;
	}

	return (*s1 - *s2);
}
