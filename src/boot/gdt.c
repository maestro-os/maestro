#include <gdt.h>
#include <libc/string.h>

/*
 * Copies the GDT to its physical address.
 */
ATTR_SECTION(".boot.text")
void gdt_copy(void)
{
	const char *src;
	char *dest;
	size_t i = 0, len;

	src = (const char *) &gdt_start;
	dest = GDT_PHYS_PTR;
	len = GDT_SIZE;
	while(i < len)
	{
		dest[i] = src[i];
		++i;
	}
}
