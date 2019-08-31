#include <memory/memory.h>

size_t memory_maps_size = 0;
size_t memory_maps_entry_size = 0;
void *memory_maps = NULL;

void *memory_end;
void *heap_begin, *heap_end;
size_t available_memory;

static int is_valid_entry(const multiboot_mmap_entry_t *entry)
{
	return (entry->addr + entry->len < ((uint64_t) 1 << (4 * 8)));
}

__attribute__((hot))
void memmap_print(void)
{
	const multiboot_mmap_entry_t *t;

	printf("--- Memory mapping ---\n");
	printf("<begin> <end> <type>\n");
	if(!memory_maps)
		return;
	t = memory_maps;
	while((void *) t < memory_maps + memory_maps_size)
	{
		if(is_valid_entry(t))
			printf("- %p %p %s\n", (void *) (uintptr_t) t->addr,
				(void *) (uintptr_t) t->addr + t->len, memmap_type(t->type));
		t = (void *) t + memory_maps_entry_size;
	}
}

__attribute__((cold))
static void *get_memory_end(void)
{
	void *end = NULL;
	const multiboot_mmap_entry_t *t;

	if(!memory_maps)
		return NULL;
	t = memory_maps;
	while((void *) t < memory_maps + memory_maps_size)
	{
		if(is_valid_entry(t) && t->addr + t->len > (uintptr_t) end)
			end = (void *) (uintptr_t) t->addr + t->len;
		t = (void *) t + memory_maps_entry_size;
	}
	return ALIGN_DOWN(end, PAGE_SIZE);
}

__attribute__((cold))
void memmap_init(const boot_info_t *info,
	void *multiboot_ptr, void *kernel_end)
{
	void *multiboot_tags_end;

	multiboot_tags_end = multiboot_ptr + multiboot_tags_size(multiboot_ptr);

	memory_maps_size = info->memory_maps_size;
	memory_maps_entry_size = info->memory_maps_entry_size;
	memory_maps = info->memory_maps;

	memory_end = get_memory_end();
	heap_begin = ALIGN_UP(MAX(multiboot_tags_end, kernel_end), PAGE_SIZE);
	heap_end = ALIGN_DOWN((void *) (info->mem_upper * 1024), PAGE_SIZE);
	if(heap_begin >= heap_end)
		PANIC("Invalid memory map!", 0);
	available_memory = heap_end - heap_begin;
}

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
