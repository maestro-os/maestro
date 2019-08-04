#include <memory/memory.h>

// TODO Separate bad ram and reserved?
static size_t get_reserved_memory(void)
{
	size_t i = 0, n;
	const multiboot_mmap_entry_t *t;

	if(!memory_maps)
		return 0;
	n = 0;
	for(; i < memory_maps_count; ++i)
	{
		t = memory_maps + i;
		if(t->type != MULTIBOOT_MEMORY_AVAILABLE)
			n += t->len;
	}
	return n;
}

void get_memory_usage(mem_usage_t *usage)
{
	size_t remaining;

	if(!usage)
		return;
	bzero(usage, sizeof(mem_usage_t));
	remaining = (size_t) heap_end;
	usage->reserved = get_reserved_memory();
	remaining -= usage->reserved;
	usage->system = (size_t) buddy_begin - 0x100000;
	remaining -= usage->system;
	// TODO Add every allocated block for `allocated`
	remaining -= usage->allocated;
	// TODO `swap`
	remaining -= usage->swap;
	usage->free = remaining;
}
