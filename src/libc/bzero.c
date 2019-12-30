#include <libc/string.h>
#include <util/attr.h>

ATTR_HOT
void bzero(void *s, size_t n)
{
	void *end;

	end = s + n;
	while(s < end && (((intptr_t) s & (sizeof(long) - 1)) != 0))
		*((char *) s++) = 0;
	while(s < (void *) ((intptr_t) end & ~((intptr_t) 7))
		&& (((intptr_t) s & (sizeof(long) - 1)) == 0))
		*((long *) s++) = 0;
	while(s < end)
		*((char *) s++) = 0;
}
