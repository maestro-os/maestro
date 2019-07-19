#include <libc/stdlib.h>
#include <libc/errno.h>

void *reallocarray(void *ptr, size_t nmemb, size_t size)
{
	const size_t s = nmemb * size;

	if(nmemb != 0 && s / nmemb != size)
	{
		errno = ENOMEM;
		return NULL;
	}

	return realloc(ptr, s);
}
