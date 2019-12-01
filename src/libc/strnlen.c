#include <libc/string.h>

// TODO Rewrite
size_t strnlen(const char *s, const size_t maxlen)
{
	size_t n = 0;

	while(s[n] && n < maxlen)
		++n;
	return n;
}
