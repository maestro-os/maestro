#include <util/util.h>

int ptr_cmp(void *p0, void *p1)
{
	return (uintptr_t) p1 - (uintptr_t) p0;
}
