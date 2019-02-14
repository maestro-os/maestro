#include "stdlib.h"

void *reallocarray(void *ptr, size_t nmemb, size_t size)
{
	const size_t s = nmemb * size;
	if(nmemb != 0 && s / nmemb != size) return NULL;

	return realloc(ptr, s);
}
