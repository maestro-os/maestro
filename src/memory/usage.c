#include <memory/memory.h>
#ifdef KERNEL_DEBUG
# include <libc/stdio.h>
#endif

// TODO Separate bad ram and reserved?
static size_t get_reserved_memory(void)
{
	size_t n = 0;
	const multiboot_mmap_entry_t *t;

	if(!memory_maps)
		return 0;
	t = memory_maps;
	while((void *) t < memory_maps + memory_maps_size)
	{
		if(t->addr + t->len < ((uint64_t) 1 << (4 * 8))
			&& t->type != MULTIBOOT_MEMORY_AVAILABLE)
			n += t->len;
		t = (void *) t + memory_maps_entry_size;
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
	usage->allocated = allocated_pages() * PAGE_SIZE;
	remaining -= usage->allocated;
	// TODO `swap`
	remaining -= usage->swap;
	usage->free = remaining;
}

#ifdef KERNEL_DEBUG
void print_mem_usage(void)
{
	mem_usage_t usage;
	size_t total;

	get_memory_usage(&usage);
	total = (size_t) heap_end;
	// TODO Use %zu and print floats
	printf("--- Memory usage ---\n");
	printf("total: %i bytes\n", (int) total);
	printf("reserved: %i bytes (%i%%)\n", (int) usage.reserved,
		(int) ((float) usage.reserved / total * 100));
	printf("system: %i bytes (%i%%)\n", (int) usage.system,
		(int) ((float) usage.system / total * 100));
	printf("allocated: %i bytes (%i%%)\n", (int) usage.allocated,
		(int) ((float) usage.allocated / total * 100));
	printf("swap: %i bytes (%i%%)\n", (int) usage.swap,
		(int) ((float) usage.swap / total * 100));
	printf("free: %i bytes (%i%%)\n", (int) usage.free,
		(int) ((float) usage.free / total * 100));
}
#endif
