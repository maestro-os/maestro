#include "stdlib.h"

void *calloc(size_t nmemb, size_t size)
{
	const size_t s = nmemb * size;

	void *ptr;
	if((ptr = malloc(s))) bzero(ptr, s);

	return ptr;
}
