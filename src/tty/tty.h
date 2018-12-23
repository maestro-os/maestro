#ifndef TTY_H
# define TTY_H

# include "../libc/string.h"

void tty_init();
void tty_write(const char* buffer, const size_t size);

#endif
