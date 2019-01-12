#ifndef KERNEL_H
# define KERNEL_H

# include "multiboot.h"
# include "libc/string.h"

# define KERNEL_MAGIC

__attribute__((noreturn))
void panic(const char* reason);

__attribute__((noreturn))
void kernel_halt();

uint8_t inb(const uint16_t port);
void outb(const uint16_t port, const uint8_t value);

#endif
