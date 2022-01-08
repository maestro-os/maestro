#include <stddef.h>
#include <stdint.h>

/*
 * Fills a field with the given value to write several bytes at a time.
 */
static long make_field(const int c)
{
	long field = 0;
	size_t i = 0;

	for(; i < sizeof(long); ++i)
		field = (field << 8) | (c & 0xff);
	return field;
}

void *memset(void *s, int c, size_t n)
{
	void *begin = s;
	void *end = begin + n;
	long field;

	while(s < end && (((intptr_t) s & (sizeof(long) - 1)) != 0))
		*((char *) s++) = c;
	field = make_field(c);
	while(s < (void *) ((intptr_t) end & ~((intptr_t) 7))
		&& (((intptr_t) s & (sizeof(long) - 1)) == 0))
		*((long *) s++) = field;
	while(s < end)
		*((char *) s++) = c;
	return begin;
}
