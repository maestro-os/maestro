#include <libc/string.h>
#include <memory/memory.h>
#include <util/util.h>

char *strndup(const char *s, const size_t n)
{
	size_t len;
	char *buff;

	if(!s || n == 0)
		return NULL;
	if((len = MIN(strlen(s), n)) == 0 || !(buff = kmalloc(len + 1)))
		return NULL;
	memcpy(buff, s, len);
	buff[len] = '\0';
	return buff;
}
