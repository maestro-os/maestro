#include <util/util.h>

/*
 * Swaps the pointers pointed by `p0` and `p1`.
 */
void swap_ptr(void **p0, void **p1)
{
	void *tmp;

	tmp = *p0;
	*p0 = *p1;
	*p1 = tmp;
}
