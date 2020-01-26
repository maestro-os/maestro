#include <libc/string.h>
#include <libc/libc_internal.h>
#include <util/util.h>

ATTR_HOT
void *memset(void *s, int c, size_t n)
{
	void *begin = s;
	void *end = begin + n;
	long field;

	field = make_field(c);
	while(s < end && (((intptr_t) s & (sizeof(long) - 1)) != 0))
		*((char *) s++) = c;
	while(s < (void *) ((intptr_t) end & ~((intptr_t) 7))
		&& (((intptr_t) s & (sizeof(long) - 1)) == 0))
		*((long *) s++) = field;
	while(s < end)
		*((char *) s++) = c;
	return begin;
}
