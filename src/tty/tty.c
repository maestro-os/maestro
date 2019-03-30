#include "tty.h"

static void init_ttys()
{
	bzero(ttys, sizeof(ttys));

	for(size_t i = 0; i < TTYS_COUNT; ++i)
		ttys[i].current_color = VGA_DEFAULT_COLOR;
}

void tty_init()
{
	init_ttys();
	switch_tty(0);

	tty_clear();
	vga_enable_cursor();
}

void tty_clear()
{
	bzero(&current_tty->history, sizeof(current_tty->history));
	vga_clear();

	current_tty->cursor_x = 0;
	current_tty->cursor_y = 0;
}

void tty_move_cursor(vgapos_t *x, vgapos_t *y)
{
	if(*x >= VGA_WIDTH)
	{
		*x = 0;
		++(*y);
	}

	vga_move_cursor(*x, *y);
}

void tty_putchar(const char c, vgapos_t *cursor_x, vgapos_t *cursor_y)
{
	vga_putchar_color(c, current_tty->current_color, *cursor_x, *cursor_y);

	++(*cursor_x);
	tty_move_cursor(cursor_x, cursor_y);
}

void tty_write(const char *buffer, const size_t count)
{
	if(!buffer || count == 0) return;

	vgapos_t *cursor_x = &current_tty->cursor_x;
	vgapos_t *cursor_y = &current_tty->cursor_y;

	// TODO Scrolling
	for(size_t i = 0; i < count; ++i)
	{
		switch(buffer[i])
		{
			case '\t':
			{
				*cursor_x += (TAB_SIZE - (*cursor_x % TAB_SIZE));
				tty_move_cursor(cursor_x, cursor_y);
				break;
			}

			case '\n':
			{
				*cursor_x = 0;
				++(*cursor_y);
				tty_move_cursor(cursor_x, cursor_y);
				break;
			}

			case '\r':
			{
				*cursor_x = 0;
				tty_move_cursor(cursor_x, cursor_y);
				break;
			}

			case ANSI_ESCAPE:
			{
				ansi_handle(current_tty, buffer, &i, count);
				break;
			}

			default:
			{
				tty_putchar(buffer[i], cursor_x, cursor_y);
				break;
			}
		}
	}
}
