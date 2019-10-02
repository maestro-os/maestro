#ifndef DEBUG_H
# define DEBUG_H

# include <kernel.h>
# include <process/process.h>

# include <libc/stdio.h>

# define GET_EBP(val)	asm("mov %%ebp, %0" : "=a"(val))

void print_regs(const regs_t *regs);
void print_callstack(void *ebp, size_t max_depth);

#endif
