#include "device.h"

__attribute__((hot))
static void keyboard_handler(const uint8_t code)
{
	// TODO
	printf("%u\n", (unsigned) code);
}

__attribute__((cold))
void keyboard_init(void)
{
	ps2_set_keyboard_hook(keyboard_handler);
}
