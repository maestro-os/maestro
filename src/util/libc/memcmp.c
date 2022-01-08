#include <stdint.h>
#include <stddef.h>

#include "libc.h"

int memcmp(const void *s1, const void *s2, size_t n)
{
	// The index of the current byte
	size_t i;
	// The end of the aligned portion of memory
	size_t align_end;

	i = 0;
	align_end = (size_t) (DOWN_ALIGN(s1 + n, sizeof(long)) - s1);
	while (i < n
		&& !(IS_ALIGNED(s1, sizeof(long)) && IS_ALIGNED(s2, sizeof(long)))
		&& ((volatile char *) s1)[i] == ((volatile char *) s2)[i])
		++i;
	while (i < align_end
		&& *((volatile long *) (s1 + i)) == *((volatile long *) (s2 + i)))
		++i;
	while (i < n
		&& ((volatile char *) s1)[i] == ((volatile char *) s2)[i])
		++i;
	if (i >= n)
		return 0;
	return (((unsigned char *) s1)[i] - ((unsigned char *) s2)[i]);
}
