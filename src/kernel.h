#ifndef KERNEL_H
# define KERNEL_H

# include "multiboot.h"
# include "libc/string.h"

# define KERNEL_VERSION	"0.1"
# define KERNEL_MAGIC

typedef struct
{
	const char *name;
	void (*init_func)();
} driver_t;

uint8_t inb(const uint16_t port);
void outb(const uint16_t port, const uint8_t value);

void error_handler(const int error);

__attribute__((noreturn))
void panic(const char *reason);

__attribute__((noreturn))
void kernel_halt();

#endif
