#include "util.h"

bool is_aligned(const void *ptr, const size_t n)
{
	return ((uintptr_t) ptr % n == 0);
}

void *align(void *ptr, const size_t n)
{
	return (ptr + ((uintptr_t) ptr % n));
}
