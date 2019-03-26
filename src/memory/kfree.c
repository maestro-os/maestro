#include "memory.h"

void kfree(void *ptr)
{
	if(!ptr) return;

	// TODO Clean memory?
	// TODO Mark memory as free
	(void) ptr;
}
