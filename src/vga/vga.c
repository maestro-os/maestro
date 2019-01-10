#include "../kernel.h"
#include "vga.h"

void vga_clear()
{
	*((uint16_t*) VGA_BUFFER) = 0;
	bzero((void*) VGA_BUFFER, VGA_WIDTH * VGA_HEIGHT * sizeof(uint16_t));
}

void vga_move_cursor(const unsigned short x, const unsigned short y)
{
	const uint16_t pos = y * VGA_WIDTH + x;
 
	outb(0x3d4, 0x0f);
	outb(0x3d5, (uint8_t) (pos & 0xff));

	outb(0x3d4, 0x0e);
	outb(0x3d5, (uint8_t) ((pos >> 8) & 0xff));
}

void vga_putchar_color(const char c, const uint8_t color,
	const size_t x, const size_t y)
{
	((uint16_t*) VGA_BUFFER)[y * VGA_WIDTH + x]
		= (uint16_t) c | ((uint16_t) color << 8);
}
