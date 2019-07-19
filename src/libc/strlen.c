#include <libc/string.h>

// TODO Rewrite
size_t strlen(const char *s)
{
	size_t i = 0;
	while(s[i]) ++i;

	return i;
}
