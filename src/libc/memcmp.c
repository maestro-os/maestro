#include "string.h"

// TODO Rewrite
__attribute__((hot))
int memcmp(const void *s1, const void *s2, size_t n)
{
	size_t i = 0;
	while(((char *) s1)[i] && ((char *) s2)[i] && i < n) ++i;

	return (*((unsigned char *) s1) - *((unsigned char *) s2));
}
