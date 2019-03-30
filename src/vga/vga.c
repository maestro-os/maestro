#include "vga.h"

void vga_clear()
{
	bzero(VGA_BUFFER, VGA_BUFFER_SIZE);
}

void vga_enable_cursor()
{
	outb(0x3d4, 0x0a);
	outb(0x3d5, inb(0x3d5) & 0xc0);

	outb(0x3d4, 0x0b);
	outb(0x3d5, (inb(0x3d5) & 0xe0) | 15);
}

void vga_disable_cursor()
{
	outb(0x3D4, 0x0A);
	outb(0x3D5, 0x20);
}

void vga_move_cursor(const vgapos_t x, const vgapos_t y)
{
	const uint16_t pos = y * VGA_WIDTH + x;
 
	outb(0x3d4, 0x0f);
	outb(0x3d5, (uint8_t) (pos & 0xff));

	outb(0x3d4, 0x0e);
	outb(0x3d5, (uint8_t) ((pos >> 8) & 0xff));

	vga_enable_cursor();
}

void vga_putchar_color(const char c, const uint8_t color,
	const vgapos_t x, const vgapos_t y)
{
	((uint16_t *) VGA_BUFFER)[y * VGA_WIDTH + x]
		= (uint16_t) c | ((uint16_t) color << 8);
}
