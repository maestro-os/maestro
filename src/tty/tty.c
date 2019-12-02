#include <tty/tty.h>
#include <pit/pit.h>
#include <libc/string.h>

// TODO Sanity checks
// TODO Implement streams and termcaps

tty_t ttys[TTYS_COUNT];
tty_t *current_tty = NULL;

__attribute__((cold))
void tty_init(void)
{
	size_t i;

	bzero(ttys, sizeof(ttys));
	for(i = 0; i < TTYS_COUNT; ++i)
	{
		ttys[i].current_color = VGA_DEFAULT_COLOR;
		ttys[i].update = 1;
		tty_clear(ttys + i);
	}
	tty_switch(0);
}

__attribute__((hot))
static inline void update_tty(tty_t *tty)
{
	vgapos_t y;

	if(tty != current_tty)
		return;
	if(tty->screen_y + VGA_HEIGHT <= HISTORY_LINES)
		memcpy(VGA_BUFFER, tty->history + (VGA_WIDTH * tty->screen_y),
			VGA_WIDTH * VGA_HEIGHT * sizeof(uint16_t));
	else
		memcpy(VGA_BUFFER, tty->history + (VGA_WIDTH * tty->screen_y),
			VGA_WIDTH * (HISTORY_LINES - tty->screen_y) * sizeof(uint16_t));
	y = tty->cursor_y - tty->screen_y;
	if(y >= 0 && y < VGA_HEIGHT)
	{
		vga_move_cursor(tty->cursor_x, y);
		vga_enable_cursor(); // TODO Enable before?
	}
	else
		vga_disable_cursor();
}

__attribute__((hot))
void tty_switch(const uint8_t tty)
{
	current_tty = ttys + tty;
	vga_enable_cursor();
	vga_move_cursor(0, 0);
	update_tty(current_tty);
}

__attribute__((hot))
void tty_reset_attrs(tty_t *tty)
{
	spin_lock(&tty->spinlock);
	tty->current_color = VGA_DEFAULT_COLOR;
	// TODO
	spin_unlock(&tty->spinlock);
}

// TODO Already locked if called from ANSI code
__attribute__((hot))
void tty_set_fgcolor(tty_t *tty, const vgacolor_t color)
{
	spin_lock(&tty->spinlock);
	tty->current_color &= ~((vgacolor_t) 0xff);
	tty->current_color |= color;
	spin_unlock(&tty->spinlock);
}

// TODO Already locked if called from ANSI code
__attribute__((hot))
void tty_set_bgcolor(tty_t *tty, const vgacolor_t color)
{
	spin_lock(&tty->spinlock);
	tty->current_color &= ~((vgacolor_t) (0xff << 4));
	tty->current_color |= color << 4;
	spin_unlock(&tty->spinlock);
}

__attribute__((hot))
static void tty_clear_portion(uint16_t *ptr, const size_t size)
{
	size_t i;

	// TODO Optimization
	for(i = 0; i < size; ++i)
		ptr[i] = EMPTY_CHAR;
}

__attribute__((hot))
void tty_clear(tty_t *tty)
{
	spin_lock(&tty->spinlock);
	tty->cursor_x = 0;
	tty->cursor_y = 0;
	tty->screen_y = 0;
	tty_clear_portion(tty->history, sizeof(tty->history) / sizeof(uint16_t));
	update_tty(tty);
	spin_unlock(&tty->spinlock);
}

__attribute__((hot))
static void tty_fix_pos(tty_t *tty)
{
	vgapos_t p;
	size_t diff, size;

	if(tty->cursor_x < 0)
	{
		p = -tty->cursor_x;
		tty->cursor_x = VGA_WIDTH - (p % VGA_WIDTH);
		tty->cursor_y += p / VGA_WIDTH - 1;
	}
	if(tty->cursor_x >= VGA_WIDTH)
	{
		p = tty->cursor_x;
		tty->cursor_x = p % VGA_WIDTH;
		tty->cursor_y += p / VGA_WIDTH;
	}
	if(tty->cursor_y < tty->screen_y)
		tty->screen_y = tty->cursor_y;
	if(tty->cursor_y >= tty->screen_y + VGA_HEIGHT)
		tty->screen_y = tty->cursor_y - VGA_HEIGHT + 1;
	if(tty->cursor_y >= HISTORY_LINES)
		tty->cursor_y = HISTORY_LINES - 1;
	if(tty->screen_y < 0)
		tty->screen_y = 0;
	if(tty->screen_y + VGA_HEIGHT > HISTORY_LINES)
	{
		diff = (tty->screen_y + VGA_HEIGHT - HISTORY_LINES) * VGA_WIDTH;
		size = sizeof(tty->history) - (diff * sizeof(uint16_t));
		memmove(tty->history, tty->history + diff, size);
		tty_clear_portion(tty->history + (size / sizeof(uint16_t)), diff);
		tty->screen_y = HISTORY_LINES - VGA_HEIGHT;
	}
}

__attribute__((hot))
static void tty_cursor_forward(tty_t *tty, const size_t x, const size_t y)
{
	tty->cursor_x += x;
	tty->cursor_y += y;
	tty_fix_pos(tty);
}

__attribute__((hot))
static void tty_cursor_backward(tty_t *tty, const size_t x, const size_t y)
{
	tty->cursor_x -= x;
	tty->cursor_y -= y;
	tty_fix_pos(tty);
}

__attribute__((hot))
static void tty_newline(tty_t *tty)
{
	tty->cursor_x = 0;
	++(tty->cursor_y);
	tty_fix_pos(tty);
}

__attribute__((hot))
static void tty_putchar(const char c, tty_t *tty)
{
	switch(c)
	{
		case '\b':
		{
			// TODO Beep
			break;
		}

		case '\t':
		{
			tty_cursor_forward(tty, GET_TAB_SIZE(tty->cursor_x), 0);
			break;
		}

		case '\n':
		{
			tty_newline(tty);
			break;
		}

		case '\r':
		{
			tty->cursor_x = 0;
			break;
		}

		default:
		{
			tty->history[HISTORY_POS(tty->cursor_x, tty->cursor_y)]
				= (uint16_t) c | ((uint16_t) tty->current_color << 8);
			tty_cursor_forward(tty, 1, 0);
			break;
		}
	}
	tty_fix_pos(tty);
	if(tty->update)
		update_tty(tty);
}

__attribute__((hot))
void tty_write(const char *buffer, const size_t count, tty_t *tty)
{
	size_t i;

	spin_lock(&tty->spinlock);
	if(!buffer || count == 0 || !tty)
	{
		spin_unlock(&tty->spinlock);
		return;
	}
	for(i = 0; i < count; ++i)
	{
		if(buffer[i] != ANSI_ESCAPE)
			tty_putchar(buffer[i], tty);
		// TODO Handle ANSI + spinlocks
		/*else
			ansi_handle(tty, buffer, &i, count);*/
		update_tty(tty);
	}
	spin_unlock(&tty->spinlock);
}

__attribute__((hot))
void tty_erase(tty_t *tty, size_t count)
{
	vgapos_t begin;
	size_t i;

	spin_lock(&tty->spinlock);
	if(tty->prompted_chars == 0)
	{
		spin_unlock(&tty->spinlock);
		return;
	}
	if(count > tty->prompted_chars)
		count = tty->prompted_chars;
	// TODO Tabs
	tty_cursor_backward(tty, count, 0);
	begin = HISTORY_POS(tty->cursor_x, tty->cursor_y); // TODO Overflow causes page fault
	for(i = begin; i < begin + count; ++i)
		tty->history[i] = EMPTY_CHAR;
	if(tty->update)
		update_tty(tty);
	tty->prompted_chars -= count;
	spin_unlock(&tty->spinlock);
}

// TODO Spinlock
__attribute__((hot))
void tty_input_hook(const key_code_t code)
{
	char c;

	if(keyboard_is_ctrl_pressed())
	{
		switch(code)
		{
			case KEY_Q:
			{
				current_tty->update = 1;
				update_tty(current_tty);
				break;
			}

			case KEY_W:
			{
				// TODO Multiple lines
				tty_erase(current_tty, current_tty->prompted_chars);
				break;
			}

			case KEY_S:
			{
				current_tty->update = 0;
				break;
			}

			// TODO
		}
		return;
	}
	c = keyboard_get_char(code, keyboard_is_shifting());
	tty_putchar(c, current_tty);
	if(c == '\n')
		current_tty->prompted_chars = 0;
	else
		++(current_tty->prompted_chars);
}

// TODO Handle cursor when scrolling up/down
// TODO Spinlock
__attribute__((hot))
void tty_ctrl_hook(const key_code_t code)
{
	// TODO Check if update is enabled?
	if(keyboard_is_shift_pressed())
	{
		if(code == KEY_PAGE_UP)
		{
			if(current_tty->screen_y > 0)
			{
				--(current_tty->screen_y);
				update_tty(current_tty);
			}
		}
		else if(code == KEY_PAGE_DOWN)
		{
			if(current_tty->screen_y + VGA_HEIGHT < HISTORY_LINES)
			{
				++(current_tty->screen_y);
				update_tty(current_tty);
			}
		}
	}
}

__attribute__((hot))
void tty_erase_hook(void)
{
	tty_erase(current_tty, 1);
}
