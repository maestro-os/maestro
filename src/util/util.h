#ifndef UTIL_H
# define UTIL_H

# include "../libc/string.h"

# define BIT_SIZEOF(expr)	(sizeof(expr) * 8)

bool is_aligned(const void *ptr, const size_t n);
void *align(void *ptr, const size_t n);

unsigned pow2(const unsigned n);

int bitmap_get(char *bitmap, const size_t index);
void bitmap_set(char *bitmap, const size_t index);
void bitmap_clear(char *bitmap, const size_t index);
void bitmap_set_range(char *bitmap, const size_t begin, const size_t end);
void bitmap_clear_range(char *bitmap, const size_t begin, const size_t end);

#endif
