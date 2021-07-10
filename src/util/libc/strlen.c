#include <stddef.h>

#include "libc.h"

/*
 * Returns the length of the string `s`.
 */
size_t strlen(const char *s)
{
	size_t i = 0;

	while(s[i])
		++i;
	return i;
}
