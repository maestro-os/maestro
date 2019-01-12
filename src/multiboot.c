#include "multiboot.h"

void read_boot_tags(const void* ptr)
{
	const multiboot_tag_t* tag = ptr + 8;

	for(; tag->type != MULTIBOOT_TAG_TYPE_END; tag += (tag->size + 7) & ~7) {
		// TODO
	}
}
