#ifndef DEBUG_H
# define DEBUG_H

# include <libc/string.h>

# define GET_ESP(val)	asm("mov %%esp, %0" : "=a"(val))
# define GET_EBP(val)	asm("mov %%ebp, %0" : "=a"(val))

# ifdef KERNEL_DEBUG
#  define debug_assert(x, str)	if(!(x)) _debug_assert_fail(str)
# else
#  define debug_assert(x, str)
# endif

typedef struct regs regs_t;
typedef volatile int spinlock_t;

typedef struct profiler_func
{
	struct profiler_func *next;

	const char *name;
	size_t count;
} profiler_func_t;

void _debug_assert_fail(const char *str);

void *_debug_sanity_check(const void *ptr);

void print_regs(const regs_t *regs);
void print_memory(const char *src, size_t n);

const char *get_function_name(void *i);
void print_callstack(void *ebp, size_t max_depth);

void profiler_capture(void);
void profiler_print(void);

void debug_spin_lock(spinlock_t *spinlock, const char *file, size_t line);
void debug_spin_unlock(spinlock_t *spinlock, const char *file, size_t line);

#endif
