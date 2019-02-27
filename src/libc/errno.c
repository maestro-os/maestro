#include "errno.h"

errno_t *__errno_location()
{
	static errno_t e;
	// TODO

	return &e;
}
