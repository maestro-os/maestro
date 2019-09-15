#ifndef DEBUG_H
# define DEBUG_H

# include <kernel.h>

# define GET_EBP(val)	asm("mov %%ebp, %0" : "=a"(val))

void print_callstack(void *ebp, size_t max_depth);

#endif
