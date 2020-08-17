#include <kernel.h>
#include <memory/memory.h>
#include <libc/stdio.h>

/*
 * Variable containing the memory mapping.
 */
memory_info_t mem_info;

/*
 * Tells if a Multiboot mmap entry is valid.
 */
ATTR_HOT
static inline int is_valid_entry(const multiboot_mmap_entry_t *entry)
{
	return (entry->addr + entry->len < ((uint64_t) 1 << (4 * 8)));
}

/*
 * Prints the memory mapping.
 */
ATTR_HOT
void memmap_print(void)
{
	const multiboot_mmap_entry_t *t;

	printf("--- Memory mapping ---\n");
	printf("<begin> <end> <type>\n");
	if(!mem_info.memory_maps)
		return;
	t = mem_info.memory_maps;
	while((void *) t < mem_info.memory_maps + mem_info.memory_maps_size)
	{
		if(is_valid_entry(t))
			printf("- %p %p %s\n", (void *) (uintptr_t) t->addr,
				(void *) (uintptr_t) t->addr + t->len, memmap_type(t->type));
		t = (void *) t + mem_info.memory_maps_entry_size;
	}
}

/*
 * Returns a pointer to the end of the system memory.
 */
ATTR_COLD
static void *get_memory_end(void)
{
	void *end = NULL;
	const multiboot_mmap_entry_t *t;

	if(!mem_info.memory_maps)
		return NULL;
	t = mem_info.memory_maps;
	while((void *) t < mem_info.memory_maps + mem_info.memory_maps_size)
	{
		if(is_valid_entry(t) && t->addr + t->len > (uintptr_t) end)
			end = (void *) (uintptr_t) t->addr + t->len;
		t = (void *) t + mem_info.memory_maps_entry_size;
	}
	return DOWN_ALIGN(end, PAGE_SIZE);
}

/*
 * Uses Multiboot informations to create a memory mapping.
 */
ATTR_COLD
void memmap_init(void *multiboot_ptr)
{
	void *multiboot_tags_end;

	multiboot_tags_end = multiboot_ptr + multiboot_tags_size(multiboot_ptr);
	mem_info.memory_maps_size = boot_info.memory_maps_size;
	mem_info.memory_maps_entry_size = boot_info.memory_maps_entry_size;
	mem_info.memory_maps = boot_info.memory_maps;
	mem_info.memory_end = get_memory_end();
	mem_info.phys_alloc_begin = UP_ALIGN(MAX(multiboot_tags_end,
		KERNEL_PHYS_END), PAGE_SIZE);
	mem_info.phys_alloc_end = DOWN_ALIGN((void *) (boot_info.mem_upper * 1024),
		PAGE_SIZE);
	if(mem_info.phys_alloc_begin >= mem_info.phys_alloc_end)
		PANIC("Invalid memory map!", 0);
	mem_info.available_memory = mem_info.phys_alloc_end
		- mem_info.phys_alloc_begin;
}

/*
 * Returns the string describing a memory region according to its type.
 */
const char *memmap_type(const uint32_t type)
{
	switch(type)
	{
		case MULTIBOOT_MEMORY_AVAILABLE: return "Available";
		case MULTIBOOT_MEMORY_RESERVED: return "Reserved";
		case MULTIBOOT_MEMORY_ACPI_RECLAIMABLE: return "ACPI";
		case MULTIBOOT_MEMORY_NVS: return "Hibernate";
		case MULTIBOOT_MEMORY_BADRAM: return "Bad RAM";
	}
	return NULL;
}
