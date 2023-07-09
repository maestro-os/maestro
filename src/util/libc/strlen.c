#include <stdint.h>
#include <stddef.h>

size_t strlen(const char *s)
{
	size_t n = 0;

	// TODO optimize
	while (s[n])
		++n;
	return n;
}
