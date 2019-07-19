#include <pit/pit.h>

__attribute__((hot))
void beep(const unsigned frequency)
{
	const unsigned div = BASE_FREQUENCY / frequency;
	outb(PIT_CHANNEL_2, div & 0xff);
	outb(PIT_CHANNEL_2, (div >> 8) & 0xff);

	const uint8_t tmp = inb(BEEPER_ENABLE);
	if(tmp != (tmp | 3)) outb(BEEPER_ENABLE, tmp | 3);
}

__attribute__((hot))
void stop_beep(void)
{
	outb(BEEPER_ENABLE, inb(BEEPER_ENABLE) & 0xfc);
}

__attribute__((hot))
void beep_during(const unsigned frequency, const unsigned duration)
{
	beep(frequency);
	pit_sleep(duration);
}
