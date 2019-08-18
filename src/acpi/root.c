#include <acpi/acpi.h>

static table_handle_t handles[] = {
	{"APIC", (void *) handle_madt},
	{"FACP", (void *) handle_fadt}
	// TODO
};

static void handle_table(acpi_sdt_header_t *table)
{
	size_t i = 0;

	if(!checksum_check(table->signature, table->length))
		return;
	while(i < sizeof(handles) / sizeof(table_handle_t))
	{
		if(signature_check(table->signature, handles[i].signature))
		{
			handles[i].handle(table);
			break;
		}
		++i;
	}
}

void handle_rsdt(rsdt_t *rsdt)
{
	size_t entries;
	size_t i = 0;

	if(!rsdt)
		return;
	entries = ACPI_ENTRIES_COUNT(rsdt);
	while(i < entries)
	{
		handle_table((void *) rsdt->entries[i]);
		++i;
	}
}

void handle_xsdt(xsdt_t *xsdt)
{
	size_t entries;
	size_t i = 0;

	if(!xsdt)
		return;
	entries = ACPI_ENTRIES_COUNT(xsdt);
	while(i < entries)
	{
		handle_table((void *) (uintptr_t) xsdt->entries[i]);
		++i;
	}
}
