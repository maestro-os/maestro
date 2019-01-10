#include "../kernel.h"
#include "tty.h"
#include "../vga/vga.h"

/*static void tty_enable_cursor()
{
	outb(0x3d4, 0x0a);
	outb(0x3d5, inb(0x3d5) & 0xc0);

	outb(0x3d4, 0x0b);
	outb(0x3d5, (inb(0x3d5) & 0xc1) | 15);
}*/

void tty_init()
{
	// TODO Switch from graphical to text mode if needed

	tty_clear();
	//tty_enable_cursor();
}

void tty_clear()
{
	vga_clear();
}

void tty_move_cursor(const unsigned short x, const unsigned short y)
{
	vga_move_cursor(x, y);
}

void tty_write(const char* buffer, const size_t size)
{
	static size_t cursor_x = 0;
	static size_t cursor_y = 0;

	for(size_t i = 0; i < size; ++i) {
		switch(buffer[i]) {
			case '\n': {
				cursor_x = 0;
				++cursor_y;
				break;
			}

			case '\r': {
				cursor_x = 0;
				break;
			}

			default: {
				tty_move_cursor(cursor_x, cursor_y);
				vga_putchar(buffer[i], cursor_x, cursor_y);

				if(cursor_x + 1 < VGA_WIDTH) {
					++cursor_x;
				} else {
					cursor_x = 0;
					++cursor_y;
				}

				break;
			}
		}
	}
}
