#include <acpi/acpi.h>
#include <libc/stdio.h>

void acpi_init(void)
{
	rsdp_desc_t *rsdp;

	if(!(rsdp = rsdp_find()))
	{
		printf("RSDP table not found.\n");
		return;
	}
	if(rsdp->revision == 2)
		handle_xsdt((void *) (uintptr_t)
			((rsdp20_desc_t *) rsdp)->xsdt_address);
	else
		handle_rsdt((void *) rsdp->rsdt_address);
}
