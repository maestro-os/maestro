#include "../kernel.h"
#include "framebuffer.h"

void vga_clear()
{
	for(size_t y = 0; y < boot_info.framebuffer_height; ++y) {
		for(size_t x = 0; x < boot_info.framebuffer_width; ++x) {
			vga_putchar(' ', x, y);
		}
	}
}

void vga_putchar_color(const char c, const uint8_t color,
	const size_t x, const size_t y)
{
	const uint16_t i = y * boot_info.framebuffer_width + x;
	*((uint16_t*) boot_info.framebuffer_addr + i)
		= (uint16_t) c | ((uint16_t) color << 8);
}
