#include <cpu/cpu.h>

void cpuid(void)
{
	char manufacturer[MANUFACTURER_ID_LENGTH + 1];
	uint8_t highest_call;

	bzero(manufacturer, sizeof(manufacturer));
	cpuid_init(&highest_call, manufacturer);

	printf("CPU manufacturer: %s\n", manufacturer);
	// TODO
}
