#include "../kernel.h"
#include "vga.h"

void vga_clear()
{
	*((uint16_t*) VGA_BUFFER) = 0;
	bzero((void*) VGA_BUFFER, VGA_WIDTH * VGA_HEIGHT * sizeof(uint16_t));
}

void vga_putchar_color(const char c, const uint8_t color,
	const size_t x, const size_t y)
{
	((uint16_t*) VGA_BUFFER)[y * VGA_WIDTH + x]
		= (uint16_t) c | ((uint16_t) color << 8);
}
