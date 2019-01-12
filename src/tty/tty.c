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
	cursor_x = 0;
	cursor_y = 0;
}

void tty_move_cursor(size_t* x, size_t* y)
{
	if(*x >= VGA_WIDTH) {
		*x = 0;
		++(*y);
	}

	vga_move_cursor(*x, *y);
}

void tty_putchar(const char c, size_t* cursor_x, size_t* cursor_y)
{
	vga_putchar(c, *cursor_x, *cursor_y);

	++(*cursor_x);
	tty_move_cursor(cursor_x, cursor_y);
}

// TODO tty_putstr

void tty_write(const char* buffer, const size_t size)
{
	// TODO Scrolling
	for(size_t i = 0; i < size; ++i) {
		switch(buffer[i]) {
			case '\t': {
				cursor_x += (TAB_SIZE - (cursor_x % TAB_SIZE));
				tty_move_cursor(&cursor_x, &cursor_y);
				break;
			}

			case '\n': {
				cursor_x = 0;
				++cursor_y;
				tty_move_cursor(&cursor_x, &cursor_y);
				break;
			}

			case '\r': {
				cursor_x = 0;
				tty_move_cursor(&cursor_x, &cursor_y);
				break;
			}

			default: {
				tty_putchar(buffer[i], &cursor_x, &cursor_y);
				break;
			}
		}
	}
}
