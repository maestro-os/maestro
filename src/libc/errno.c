#include "errno.h"

errno_t *__errno_location(void)
{
	static errno_t e;
	// TODO

	return &e;
}
