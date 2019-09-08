#include <libc/string.h>

char *strcpy(char *dest, const char *src)
{
	return memcpy(dest, src, strlen(src) + 1);
}
