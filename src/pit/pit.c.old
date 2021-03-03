#include <pit/pit.h>
#include <idt/idt.h>

/*
 * The current frequency of the PIT in hertz.
 */
static unsigned current_frequency;

/*
 * Initializes the PIT.
 * This function disables interrupts.
 */
ATTR_COLD
void pit_init(void)
{
	CLI();
	outb(PIT_COMMAND, PIT_SELECT_CHANNEL_0 | PIT_ACCESS_LOBYTE_HIBYTE
		| PIT_MODE_4);
	outb(PIT_COMMAND, PIT_SELECT_CHANNEL_2 | PIT_ACCESS_LOBYTE_HIBYTE
		| PIT_MODE_4);
}

/*
 * Sets the PIT divider value.
 */
ATTR_HOT
void pit_set_count(const uint16_t count)
{
	CLI();
	outb(PIT_CHANNEL_0, count & 0xff);
	outb(PIT_CHANNEL_0, (count >> 8) & 0xff);
}

/*
 * Sets the current frequency of the PIT in hertz.
 */
ATTR_HOT
void pit_set_frequency(const unsigned frequency)
{
	unsigned c;

	current_frequency = frequency;
	if((c = CEIL_DIVISION(BASE_FREQUENCY, frequency)) & ~0xffff)
		c = 0;
	pit_set_count(c);
}
