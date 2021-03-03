#ifndef TTY_H
# define TTY_H

# include <kernel.h>
# include <vga/vga.h>
# include <keyboard/keyboard.h>
# include <libc/string.h>
# include <util/util.h>

# define TTYS_COUNT		8
// TODO uncomment # define HISTORY_LINES	128
# define HISTORY_LINES	1024

# define EMPTY_CHAR			(VGA_DEFAULT_COLOR << 8)
# define HISTORY_POS(x, y)	((y) * VGA_WIDTH + (x))

# define TAB_SIZE				4
# define GET_TAB_SIZE(cursor_x)	(TAB_SIZE - ((cursor_x) % TAB_SIZE))

# define ANSI_ESCAPE	0x1b

# define BELL_FREQUENCY	2000
# define BELL_DURATION	500

typedef struct tty
{
	vgapos_t cursor_x;
	vgapos_t cursor_y;
	vgapos_t screen_y;

	vgacolor_t current_color;

	uint16_t history[VGA_WIDTH * HISTORY_LINES];

	size_t prompted_chars;
	char update;

	spinlock_t spinlock;
} tty_t;

extern tty_t *current_tty;

void tty_init(void);

void tty_switch(uint8_t tty);
void tty_reset_attrs(tty_t *tty);
void tty_set_fgcolor(tty_t *tty, const vgacolor_t color);
void tty_set_bgcolor(tty_t *tty, const vgacolor_t color);

void tty_clear(tty_t *tty);
void tty_write(const char *buffer, const size_t count, tty_t *tty);
void tty_erase(tty_t *tty, size_t count);

void ansi_handle(tty_t *tty, const char *buffer,
	size_t *i, const size_t count);

void tty_input_hook(const key_code_t c);
void tty_ctrl_hook(const key_code_t code);
void tty_erase_hook(void);

#endif
