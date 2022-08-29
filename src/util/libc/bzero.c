#include <stddef.h>
#include <stdint.h>

#include "libc.h"

void bzero(void *s, size_t n)
{
	// The end of the memory to be written
	void *end;
	// The end of the aligned portion of memory to be written
	void *align_end;

	end = s + n;
	align_end = DOWN_ALIGN(end, sizeof(long));
	while (s < end && !IS_ALIGNED(s, sizeof(long)))
	{
		*((volatile char *) s) = 0;
		s += sizeof(char);
	}
	while (s < align_end)
	{
		*((volatile long *) s) = 0;
		s += sizeof(long);
	}
	while (s < end)
	{
		*((volatile char *) s) = 0;
		s += sizeof(char);
	}
}
