#include <memory/memory.h>

#ifdef KERNEL_DEBUG
# include <libc/stdio.h>

# define TO_PERCENTAGE(n, total)	((n) * 100 / (total))
#endif

/*
 * Returns the amount of reserved memory in bytes.
 */
static size_t get_reserved_memory(void)
{
	size_t n = 0;
	const multiboot_mmap_entry_t *t;

	if(!mem_info.memory_maps)
		return 0;
	t = mem_info.memory_maps;
	while((void *) t < mem_info.memory_maps + mem_info.memory_maps_size)
	{
		if(t->addr + t->len < ((uint64_t) 1 << (4 * 8))
			&& t->type != MULTIBOOT_MEMORY_AVAILABLE)
			n += t->len;
		t = (void *) t + mem_info.memory_maps_entry_size;
	}
	return n;
}

/*
 * Fills the given structure with the memory usage.
 */
void get_memory_usage(mem_usage_t *usage)
{
	size_t remaining;

	if(!usage)
		return;
	bzero(usage, sizeof(mem_usage_t));
	remaining = (size_t) mem_info.heap_end;
	usage->reserved = get_reserved_memory();
	remaining -= usage->reserved;
	usage->bad_ram = 0; // TODO Get bad ram from mapping
	remaining -= usage->bad_ram;
	usage->system = (size_t) mem_info.heap_begin - 0x100000;
	remaining -= usage->system;
	usage->allocated = allocated_pages() * PAGE_SIZE;
	remaining -= usage->allocated;
	usage->free = remaining;
}

#ifdef KERNEL_DEBUG
/*
 * Prints the memory usage.
 */
void print_mem_usage(void)
{
	mem_usage_t usage;
	size_t total;

	get_memory_usage(&usage);
	total = (size_t) mem_info.heap_end;
	printf("--- Memory usage ---\n");
	printf("total: %zu bytes\n", total);
	printf("reserved: %zu bytes (%zu%%)\n", usage.reserved,
		TO_PERCENTAGE(usage.reserved, total));
	printf("bad ram: %zu bytes (%zu%%)\n", usage.bad_ram,
		TO_PERCENTAGE(usage.bad_ram, total));
	printf("system: %zu bytes (%zu%%)\n", usage.system,
		TO_PERCENTAGE(usage.system, total));
	printf("allocated: %zu bytes (%zu%%)\n", usage.allocated,
		TO_PERCENTAGE(usage.allocated, total));
	printf("free: %zu bytes (%zu%%)\n", usage.free,
		TO_PERCENTAGE(usage.free, total));
}
#endif
