#ifndef STRING_H
# define STRING_H

# include <stdbool.h>
# include <stddef.h>
# include <stdint.h>

// TODO Must be placed in `strings.h`
void bzero(void *s, size_t n);

int memcmp(const void* s1, const void* s2, size_t n);
void* memcpy(void* dest, const void* src, size_t n);
void* memmove(void* dest, const void* src, size_t n);
void* memset(void* s, const int c, size_t n);
size_t strlen(const char* s);

#endif
