#include <acpi/acpi.h>

static int check_acpi_mode(const fadt_t *fadt)
{
	return (fadt->smi_command_port == 0
		&& (fadt->acpi_enable == 0 && fadt->acpi_disable == 0)
			&& fadt->pm1a_control_block & 1);
}

static void acpi_mode_enable(const fadt_t *fadt)
{
	outb(fadt->smi_command_port, fadt->acpi_enable);
	while((inw(fadt->pm1a_control_block) & 1) == 0)
		;
}

void handle_fadt(fadt_t *fadt)
{
	if(!fadt || !checksum_check(fadt, fadt->header.length))
		return;
	if(!check_acpi_mode(fadt))
	{
		printf("ACPI mode not enabled. Enabling...\n");
		acpi_mode_enable(fadt);
	}
	handle_dsdt((void *) fadt->dsdt);
	// TODO
}
