#include "pit.h"

__attribute__((cold))
void pit_init()
{
	outb(PIT_COMMAND, PIT_SELECT_CHANNEL_0 | PIT_ACCESS_LOBYTE_HIBYTE
		| PIT_MODE_0);
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
