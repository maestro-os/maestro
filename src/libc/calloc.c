#include "stdlib.h"
#include "errno.h"

void *calloc(size_t nmemb, size_t size)
{
	const size_t s = nmemb * size;

	if(nmemb != 0 && s / nmemb != size)
	{
		errno = ENOMEM;
		return NULL;
	}

	void *ptr;
	if((ptr = malloc(s))) bzero(ptr, s);

	return ptr;
}
