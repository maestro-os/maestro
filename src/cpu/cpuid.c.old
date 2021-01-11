#include <cpu/cpu.h>

/*
 * Retrives informations on the CPU using `cpuid` instruction.
 */
void cpuid(void)
{
	char manufacturer[MANUFACTURER_ID_LENGTH + 1];
	uint8_t highest_call;

	if(!cpuid_available())
	{
		printf("CPUID not available\n");
		return;
	}
	bzero(manufacturer, sizeof(manufacturer));
	cpuid_init(&highest_call, manufacturer);
	printf("CPU manufacturer: %s\n", manufacturer);
}
