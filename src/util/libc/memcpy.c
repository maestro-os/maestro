#include <stdint.h>
#include <stddef.h>

#include "libc.h"

void *memcpy(void *dest, const void *src, size_t n)
{
	asm volatile("rep movsb "
		: "=D" (dest),
			"=S" (src),
			"=c" (n)
		: "0" (dest),
			"1" (src),
			"2" (n)
		: "memory");
	return dest;
}
