#ifndef DEBUG_H
# define DEBUG_H

# include <kernel.h>
# include <process/process.h>

# include <libc/stdio.h>

# define GET_ESP(val)	asm("mov %%esp, %0" : "=a"(val))
# define GET_EBP(val)	asm("mov %%ebp, %0" : "=a"(val))

typedef struct profiler_func
{
	struct profiler_func *next;

	const char *name;
	size_t count;
} profiler_func_t;

void print_regs(const regs_t *regs);
void print_memory(const char *src, size_t n);

const char *get_function_name(void *i);
void print_callstack(void *ebp, size_t max_depth);

void profiler_capture(void);
void profiler_print(void);

#endif
