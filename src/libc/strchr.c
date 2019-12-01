#include <libc/string.h>

char *strchr(const char *s, const int c)
{
	while(*s && *s != c)
		++s;
	return (*s == c ? (char *) s : NULL);
}
