#include <acpi/acpi.h>

void handle_fadt(fadt_t *fadt)
{
	if(!fadt || !checksum_check(fadt, fadt->header.length))
		return;
	handle_dsdt((void *) fadt->dsdt);
	// TODO
}
