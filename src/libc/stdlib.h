#ifndef _STDLIB_H
# define _STDLIB_H

# include <libc/string.h>

# ifdef KERNEL_MAGIC
#  define ABORT_INSTRUCTION	panic("Aborted")
# else
#  define ABORT_INSTRUCTION	exit(-127)
# endif

int atoi(const char *nptr);
long atol(const char *nptr);

void *malloc(size_t size);
void free(void *ptr);
void *calloc(size_t nmemb, size_t size);
void *realloc(void *ptr, size_t size);
void *reallocarray(void *ptr, size_t nmemb, size_t size);

void exit(int status);

__attribute__((noreturn))
void abort(void);

#endif
