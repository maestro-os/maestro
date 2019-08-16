#ifndef KERNEL_H
# define KERNEL_H

# include <multiboot.h>
# include <gdt.h>

# include <libc/string.h>
# ifdef KERNEL_DEBUG
#  include <libc/stdio.h>
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
	void (*init_func)();
} driver_t;

uint8_t inb(uint16_t port);
void outb(uint16_t port, uint8_t value);

void error_handler(unsigned error, uint32_t error_code);

__attribute__((noreturn))
extern void kernel_loop(void);

__attribute__((noreturn))
void kernel_panic(const char *reason, uint32_t code);
__attribute__((noreturn))
void kernel_panic_(const char *reason, uint32_t code,
	const char *file, int line);

__attribute__((noreturn))
extern void kernel_halt(void);

#endif
