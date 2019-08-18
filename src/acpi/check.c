#include <acpi/acpi.h>

int signature_check(const char *desc_sign, const char *sign)
{
	size_t i = 0;

	if(!desc_sign || !sign)
		return 0;
	while(i < 4)
	{
		if(desc_sign[i] != sign[i])
			return 0;
		++i;
	}
	return 1;
}

int checksum_check(void *desc, const size_t size)
{
	uint8_t sum = 0;
	size_t i;

	if(!desc || size == 0)
		return 0;
	for(i = 0; i < size; ++i)
		sum += ((const uint8_t *) desc)[i];
	return (sum == 0);
}
