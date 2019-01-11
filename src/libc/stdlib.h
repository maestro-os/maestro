#ifndef _STDLIB_H
# define _STDLIB_H

# ifdef KERNEL_MAGIC
#  define ABORT_INSTRUCTION	panic("Aborted")
# else
#  define ABORT_INSTRUCTION	// TODO
# endif

void exit(int status);

__attribute__((noreturn))
void abort();

#endif
