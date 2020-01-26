#ifndef _KMALLOC_H
# define _KMALLOC_H

# include <libc/string.h>

void *kmalloc(size_t size);
void *kmalloc_zero(size_t size);
void kfree(void *ptr);
void *krealloc(void *ptr, size_t size);

#endif
