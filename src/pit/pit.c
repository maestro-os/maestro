#include "pit.h"

static unsigned current_frequency;

// TODO cli?
__attribute__((cold))
void pit_init(void)
{
	outb(PIT_COMMAND, PIT_SELECT_CHANNEL_0 | PIT_ACCESS_LOBYTE_HIBYTE
		| PIT_MODE_0);
	pit_set_frequency(1000);

	outb(PIT_COMMAND, PIT_SELECT_CHANNEL_2 | PIT_ACCESS_LOBYTE_HIBYTE
		| PIT_MODE_3);
}

// TODO cli?
__attribute__((hot))
void pit_set_count(const uint16_t count)
{
	outb(PIT_CHANNEL_0, count & 0xff);
	outb(PIT_CHANNEL_0, (count >> 8) & 0xff);
}

__attribute__((hot))
void pit_set_frequency(const unsigned frequency)
{
	current_frequency = frequency;

	unsigned c;
	if((c = UPPER_DIVISION(BASE_FREQUENCY, frequency)) & ~0xffff) c = 0;

	pit_set_count(c);
}

__attribute__((hot))
void pit_sleep(const unsigned duration)
{
	// TODO
	(void) duration;
}
