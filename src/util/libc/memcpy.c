#include <stdint.h>
#include <stddef.h>

#include "libc.h"

void *memcpy(void *dest, const void *src, size_t n)
{
	// The beginning of the destination memory
	void *begin;
	// The end of the destination memory
	void *end;
	// The end of the aligned portion of memory to be written
	void *align_end;

	begin = dest;
	end = begin + n;
	align_end = DOWN_ALIGN(end, sizeof(long));
	while (dest < end
		&& !(IS_ALIGNED(dest, sizeof(long)) && IS_ALIGNED(src, sizeof(long))))
	{
		*((volatile char *) dest) = *((volatile char *) src);
		dest += sizeof(char);
		src += sizeof(char);
	}
	while (dest < align_end)
	{
		*((volatile long *) dest) = *((volatile long *) src);
		dest += sizeof(long);
		src += sizeof(long);
	}
	while (dest < end)
	{
		*((volatile char *) dest) = *((volatile char *) src);
		dest += sizeof(char);
		src += sizeof(char);
	}
	return begin;
}
