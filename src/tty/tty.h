#ifndef TTY_H
# define TTY_H

# include "../kernel.h"
# include "../libc/string.h"
# include "../vga/vga.h"

# define TTYS_COUNT		8
# define HISTORY_LINES	128
# define TAB_SIZE		4
# define ANSI_ESCAPE	0x1b

typedef struct tty
{
	vgapos_t cursor_x;
	vgapos_t cursor_y;
	vgapos_t screen_y;

	vgacolor_t current_color;

	uint16_t history[VGA_WIDTH * HISTORY_LINES];
} tty_t;

tty_t ttys[TTYS_COUNT];
tty_t *current_tty;

inline void switch_tty(const uint8_t tty)
{
	current_tty = ttys + tty;
}

void tty_init();

void tty_reset_attrs(tty_t *tty);
void tty_set_fgcolor(tty_t *tty, const vgacolor_t color);
void tty_set_bgcolor(tty_t *tty, const vgacolor_t color);

void tty_clear();
void tty_putchar(const char c, tty_t *tty, const bool update);
void tty_write(const char *buffer, const size_t count, tty_t *tty);

void ansi_handle(tty_t *tty, const char *buffer,
	size_t *i, const size_t count);

#endif
