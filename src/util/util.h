#ifndef UTIL_H
# define UTIL_H

# include "../libc/string.h"

bool is_aligned(const void *ptr, const size_t n);
void *align(void *ptr, const size_t n);

#endif
