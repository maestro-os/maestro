#include "../kernel.h"
#include "framebuffer.h"

void text_clear()
{
	for(size_t y = 0; y < TEXT_HEIGHT; ++y) {
		for(size_t x = 0; x < TEXT_WIDTH; ++x) {
			text_putchar(' ', x, y);
		}
	}
}

void text_putchar_color(const char c, const uint8_t color,
	const size_t x, const size_t y)
{
	const uint16_t i = y * TEXT_WIDTH + x;
	*((uint16_t*) TEXT_BUFFER + i) = (uint16_t) c | ((uint16_t) color << 8);
}
