#include <acpi/acpi.h>

static int signature_check(char *sign)
{
	size_t i = 0;
	size_t len;

	len = strlen(RSDP_SIGNATURE);
	while(i < len)
	{
		if(sign[i] != RSDP_SIGNATURE[i])
			return 0;
		++i;
	}
	return 1;
}

static int checksum_check(rsdp_desc_t *desc, const size_t size)
{
	uint8_t sum = 0;
	size_t i;

	for(i = 0; i < size; ++i)
		sum += ((const uint8_t *) desc)[i];
	return (sum == 0);
}

static int rsdp_check(rsdp_desc_t *desc)
{
	if(!signature_check(desc->signature))
		return 0;
	if(!checksum_check(desc, sizeof(rsdp_desc_t)))
		return 0;
	if(desc->revision == 2 && !checksum_check(desc, sizeof(rsdp20_desc_t)))
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
