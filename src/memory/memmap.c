#include <memory/memory.h>

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
