#include <vga/vga.h>

void vga_clear(void)
{
	const uint16_t c = VGA_DEFAULT_COLOR << 8;
	size_t i;

	for(i = 0; i < VGA_WIDTH * VGA_HEIGHT; ++i)
		*((uint16_t *) VGA_BUFFER + i) = c;
}

void vga_enable_cursor(void)
{
	outb(0x3d4, 0x0a);
	outb(0x3d5, (inb(0x3d5) & 0xc0) | CURSOR_START);
	outb(0x3d4, 0x0b);
	outb(0x3d5, (inb(0x3d5) & 0xe0) | CURSOR_END);
}

void vga_disable_cursor(void)
{
	outb(0x3d4, 0x0a);
	outb(0x3d5, 0x20);
}

void vga_move_cursor(const vgapos_t x, const vgapos_t y)
{
	uint16_t pos;
 
 	pos = y * VGA_WIDTH + x;
	outb(0x3d4, 0x0f);
	outb(0x3d5, (uint8_t) (pos & 0xff));
	outb(0x3d4, 0x0e);
	outb(0x3d5, (uint8_t) ((pos >> 8) & 0xff));
}

void vga_putchar_color(const char c, const uint8_t color,
	const vgapos_t x, const vgapos_t y)
{
	((uint16_t *) VGA_BUFFER)[y * VGA_WIDTH + x]
		= (uint16_t) c | ((uint16_t) color << 8);
}
