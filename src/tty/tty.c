#include "tty.h"
#include "../pit/pit.h"
#include "../libc/string.h"

__attribute__((cold))
void tty_init()
{
	bzero(ttys, sizeof(ttys));
	for(size_t i = 0; i < TTYS_COUNT; ++i)
		ttys[i].current_color = VGA_DEFAULT_COLOR;

	switch_tty(0);

	vga_enable_cursor();
	tty_clear(current_tty);
}

__attribute__((hot))
void tty_reset_attrs(tty_t *tty)
{
	tty->current_color = VGA_DEFAULT_COLOR;
	// TODO
}

__attribute__((hot))
void tty_set_fgcolor(tty_t *tty, const vgacolor_t color)
{
	tty->current_color &= ~((vgacolor_t) 0xff);
	tty->current_color |= color;
}

__attribute__((hot))
void tty_set_bgcolor(tty_t *tty, const vgacolor_t color)
{
	tty->current_color &= ~((vgacolor_t) (0xff << 4));
	tty->current_color |= color << 4;
}

__attribute__((hot))
static void tty_clear_portion(uint16_t *ptr, const size_t size)
{
	const uint16_t c = VGA_DEFAULT_COLOR << 8;

	// TODO Optimization
	for(size_t i = 0; i < size; ++i)
		ptr[i] = c;
}

__attribute__((hot))
static inline void update_tty(tty_t *tty)
{
	if(tty->screen_y + VGA_HEIGHT <= HISTORY_LINES)
		memcpy(VGA_BUFFER, tty->history + (VGA_WIDTH * tty->screen_y),
			VGA_WIDTH * VGA_HEIGHT * sizeof(uint16_t));
	else
		memcpy(VGA_BUFFER, tty->history + (VGA_WIDTH * tty->screen_y),
			VGA_WIDTH * (HISTORY_LINES - tty->screen_y) * sizeof(uint16_t));

	vga_move_cursor(tty->cursor_x, tty->cursor_y);
}

__attribute__((hot))
void tty_clear(tty_t *tty)
{
	tty->cursor_x = 0;
	tty->cursor_y = 0;
	tty->screen_y = 0;

	tty_clear_portion(tty->history, sizeof(tty->history) / sizeof(uint16_t));
	update_tty(tty);
}

__attribute__((hot))
static void tty_fix_pos(tty_t *tty)
{
	if(tty->cursor_x >= VGA_WIDTH)
	{
		const vgapos_t p = tty->cursor_x;
		tty->cursor_x = p % VGA_WIDTH;
		tty->cursor_y += p / VGA_WIDTH;
	}

	// TODO if(cursor_y < 0) -> make vgapos_t signed?

	if(tty->cursor_y >= VGA_HEIGHT)
	{
		tty->screen_y += (tty->cursor_y - VGA_HEIGHT) + 1;
		tty->cursor_y = VGA_HEIGHT - 1;
	}

	if(tty->screen_y + VGA_HEIGHT >= HISTORY_LINES)
	{
		const size_t diff = VGA_WIDTH * (tty->screen_y - HISTORY_LINES + 1);
		const size_t size = sizeof(tty->history) - (diff * sizeof(uint16_t));

		memmove(tty->history, tty->history + (diff * sizeof(uint16_t)), size);
		tty_clear_portion(tty->history + size, diff);

		tty->screen_y = HISTORY_LINES - VGA_HEIGHT + 1;
	}
}

__attribute__((hot))
void tty_putchar(const char c, tty_t *tty, const bool update)
{
	switch(c)
	{
		case '\b':
		{
			beep_during(BELL_FREQUENCY, BELL_DURATION);
			break;
		}

		case '\t':
		{
			tty->cursor_x += (TAB_SIZE - (tty->cursor_x % TAB_SIZE));
			break;
		}

		case '\n':
		{
			tty->cursor_x = 0;
			++(tty->cursor_y);
			break;
		}

		case '\r':
		{
			tty->cursor_x = 0;
			break;
		}

		default:
		{
			const vgapos_t pos = (tty->screen_y + tty->cursor_y) * VGA_WIDTH
				+ tty->cursor_x;
			tty->history[pos] = (uint16_t) c
				| ((uint16_t) tty->current_color << 8);
			++(tty->cursor_x);
			break;
		}
	}

	tty_fix_pos(tty);
	if(update) update_tty(tty);
}

__attribute__((hot))
void tty_write(const char *buffer, const size_t count, tty_t *tty)
{
	if(!buffer || count == 0 || !tty) return;

	for(size_t i = 0; i < count; ++i)
	{
		if(buffer[i] != ANSI_ESCAPE)
			tty_putchar(buffer[i], tty, false);
		else
			ansi_handle(tty, buffer, &i, count);

		update_tty(tty);
	}
}
