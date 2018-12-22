#include "kernel.h"

void vga_init()
{
	for(size_t y = 0; y < VGA_HEIGHT; ++y) {
		for(size_t x = 0; x < VGA_WIDTH; ++x) {
			vga_putchar(' ', x, y);
		}
	}
}

void vga_putchar_color(const char c, const uint8_t color,
	const size_t x, const size_t y)
{
	*((uint16_t*) VGA_BUFFER + (y * VGA_WIDTH + x))
		= (uint16_t) c | ((uint16_t) color << 8);
}

void vga_putstr_color(const char* str, const uint8_t color,
	const size_t x, const size_t y)
{
	size_t n = 0;

	while(str[n]) {
		vga_putchar_color(str[n], color, x + n, y);
		++n;
	}
}
