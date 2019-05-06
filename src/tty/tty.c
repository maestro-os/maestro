#include "tty.h"

static void init_ttys()
{
	bzero(ttys, sizeof(ttys));
	for(size_t i = 0; i < TTYS_COUNT; ++i)
		ttys[i].current_color = VGA_DEFAULT_COLOR;

	switch_tty(0);
}

void tty_init()
{
	init_ttys();
	vga_enable_cursor();
	tty_clear();
}

void tty_reset_attrs(tty_t *tty)
{
	tty->current_color = VGA_DEFAULT_COLOR;
	// TODO
}

void tty_set_fgcolor(tty_t *tty, const vgacolor_t color)
{
	tty->current_color &= ~((vgacolor_t) 0xff);
	tty->current_color |= color;
}

void tty_set_bgcolor(tty_t *tty, const vgacolor_t color)
{
	tty->current_color &= ~((vgacolor_t) (0xff << 4));
	tty->current_color |= color << 4;
}

void tty_clear()
{
	bzero(&current_tty->history, sizeof(current_tty->history));
	vga_clear();

	current_tty->cursor_x = 0;
	current_tty->cursor_y = 0;
}

static void tty_correct_pos(vgapos_t *x, vgapos_t *y)
{
	if(*x >= VGA_WIDTH)
	{
		const vgapos_t p = *x;

		*x = p % VGA_WIDTH;
		*y += p / VGA_WIDTH;
	}
}

void tty_putchar(const char c, vgapos_t *cursor_x, vgapos_t *cursor_y)
{
	switch(c)
	{
		case '\t':
		{
			*cursor_x += (TAB_SIZE - (*cursor_x % TAB_SIZE));
			break;
		}

		case '\n':
		{
			*cursor_x = 0;
			++(*cursor_y);
			break;
		}

		case '\r':
		{
			*cursor_x = 0;
			break;
		}

		default:
		{
			vga_putchar_color(c, current_tty->current_color,
				*cursor_x, *cursor_y);
			++(*cursor_x);
			break;
		}
	}

	tty_correct_pos(cursor_x, cursor_y);
	// TODO History

	vga_move_cursor(*cursor_x, *cursor_y);
	// TODO Scrolling
}

void tty_write(const char *buffer, const size_t count)
{
	if(!buffer || count == 0) return;

	vgapos_t *cursor_x = &(current_tty->cursor_x);
	vgapos_t *cursor_y = &(current_tty->cursor_y);

	for(size_t i = 0; i < count; ++i)
	{
		switch(buffer[i])
		{
			case '\t':
			{
				*cursor_x += (TAB_SIZE - (*cursor_x % TAB_SIZE));
				break;
			}

			case '\n':
			{
				*cursor_x = 0;
				++(*cursor_y);
				break;
			}

			case '\r':
			{
				*cursor_x = 0;
				break;
			}

			case ANSI_ESCAPE:
			{
				ansi_handle(current_tty, buffer, &i, count);
				break;
			}

			default:
			{
				vga_putchar_color(buffer[i], current_tty->current_color,
					*cursor_x, *cursor_y);
				++(*cursor_x);
				break;
			}
		}

		tty_correct_pos(cursor_x, cursor_y);
		// TODO History
	}

	vga_move_cursor(*cursor_x, *cursor_y);
	// TODO Scrolling
}
