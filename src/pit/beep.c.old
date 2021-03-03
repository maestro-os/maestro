#include <pit/pit.h>

/*
 * Makes the PC speaker beep at the given frequency.
 */
void beep(const unsigned frequency)
{
	unsigned div;
	uint8_t tmp;

	div = BASE_FREQUENCY / frequency;
	outb(PIT_CHANNEL_2, div & 0xff);
	outb(PIT_CHANNEL_2, (div >> 8) & 0xff);
	tmp = inb(BEEPER_ENABLE);
	if(tmp != (tmp | 3))
		outb(BEEPER_ENABLE, tmp | 3);
}

/*
 * Stops the PC speaker beep.
 */
void stop_beep(void)
{
	outb(BEEPER_ENABLE, inb(BEEPER_ENABLE) & 0xfc);
}
