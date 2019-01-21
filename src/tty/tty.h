#ifndef TTY_H
# define TTY_H

# include "../libc/string.h"

# define TAB_SIZE	4

size_t cursor_x;
size_t cursor_y;

void tty_init();
void tty_clear();
void tty_putchar(const char c, size_t *cursor_x, size_t *cursor_y);
void tty_move_cursor(size_t *x, size_t *y);
void tty_write(const char *buffer, const size_t size);

#endif
