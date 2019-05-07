#ifndef IO_H
# define IO_H

# include <stdint.h>

uint8_t inb(const uint16_t port);
void outb(const uint16_t port, const uint8_t value);

#endif
