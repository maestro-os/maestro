#include <debug/debug.h>
#include <libc/ctype.h>

void print_memory(const char *src, const size_t n)
{
	size_t i;

	// TODO Print addr/offset and hexadecimal values
	for(i = 0; i < n; ++i)
	{
		if(!isprint(src[i]))
			printf(".");
		else
			printf("%c", src[i]);
	}
}
