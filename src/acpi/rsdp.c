#include <acpi/acpi.h>

static int rsdp_check(rsdp_desc_t *desc)
{
	if(!signature_check(desc->signature, RSDP_SIGNATURE))
		return 0;
	if(!checksum_check(desc, sizeof(rsdp_desc_t)))
		return 0;
	if(desc->revision == 2
		&& !checksum_check(desc, ((rsdp20_desc_t *) desc)->length))
		return 0;
	return 1;
}

// TODO Also scan into EBDA
rsdp_desc_t *rsdp_find(void)
{
	void *i = RSDP_LOOKUP_BEGIN;

	while(i < RSDP_LOOKUP_END)
	{
		if(rsdp_check(i))
			return i;
		i += 16;
	}
	return NULL;
}
