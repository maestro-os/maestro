#include <acpi/acpi.h>

void acpi_init(void)
{
	rsdp_desc_t *rsdp;

	if(!(rsdp = rsdp_find()))
		printf("RSDP table not found.\n");
	// TODO
}
