#include <libc/string.h>
#include <memory/memory.h>

char *strdup(const char *s)
{
	size_t len;
	char *buff;

	if(!s)
		return NULL;
	if((len = strlen(s)) == 0 || !(buff = kmalloc(len + 1)))
		return NULL;
	memcpy(buff, s, len);
	buff[len] = '\0';
	return buff;
}
