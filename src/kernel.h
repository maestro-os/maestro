#ifndef KERNEL_H
# define KERNEL_H

# include "multiboot.h"
# include "io.h"
# include "libc/string.h"

# define KERNEL_VERSION	"0.1"
# define KERNEL_MAGIC

# ifdef KERNEL_DEBUG
#  define PANIC(reason)	kernel_panic_(reason, __FILE__, __LINE__)
# else
#  define PANIC(reason)	kernel_panic(reason)
# endif

typedef struct
{
	const char *name;
	void (*init_func)();
} driver_t;

void error_handler(const int error);

__attribute__((noreturn))
void kernel_panic(const char *reason);
__attribute__((noreturn))
void kernel_panic_(const char *reason, const char *file, const int line);

__attribute__((noreturn))
extern void kernel_halt(void);

#endif
