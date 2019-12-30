#ifndef KERNEL_H
# define KERNEL_H

# include <multiboot.h>
# include <gdt.h>
# include <util/util.h>
# include <util/attr.h>

# include <libc/string.h>
# ifdef KERNEL_DEBUG
#  include <libc/errno.h>
#  include <libc/stdio.h>
#  include <debug/debug.h>
# endif

# define KERNEL_VERSION	"0.1"
# define KERNEL_MAGIC

# ifdef KERNEL_DEBUG
#  define PANIC(reason, code)	kernel_panic_(reason, code, __FILE__, __LINE__)
# else
#  define PANIC(reason, code)	kernel_panic(reason, code)
# endif

typedef struct
{
	const char *name;
	void (*init_func)(void);
} driver_t;

uint8_t inb(uint16_t port);
uint16_t inw(uint16_t port);
uint32_t inl(uint16_t port);
void outb(uint16_t port, uint8_t value);
void outw(uint16_t port, uint16_t value);
void outl(uint16_t port, uint32_t value);

extern void kernel_wait(void);
ATTR_NORETURN
extern void kernel_loop(void);
ATTR_NORETURN
extern void kernel_halt(void);

void error_handler(unsigned error, uint32_t error_code);

ATTR_NORETURN
void kernel_panic(const char *reason, uint32_t code);
ATTR_NORETURN
void kernel_panic_(const char *reason, uint32_t code,
	const char *file, int line);

#endif
