#include <libc/string.h>

__attribute__((hot))
void bzero(void *s, size_t n)
{
	void *end = s + n;

	while(s < end && (((intptr_t) s & (sizeof(long) - 1)) != 0))
		*((char *) s++) = 0;
	while(s < (void *) ((intptr_t) end & ~((intptr_t) 7))
		&& (((intptr_t) s & (sizeof(long) - 1)) == 0))
		*((long *) s++) = 0;
	while(s < end)
		*((char *) s++) = 0;
}
