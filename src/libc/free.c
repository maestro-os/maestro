#include "stdlib.h"
#include "../kernel.h"

void free(void *ptr)
{
	if(!ptr) return;
	mm_free(ptr);
}
