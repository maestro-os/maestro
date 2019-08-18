#include <acpi/acpi.h>

void handle_madt(madt_t *madt)
{
	if(!madt || !checksum_check(madt, madt->header.length))
		return;
	// TODO
}
