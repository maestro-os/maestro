#ifndef _STRING_H
# define _STRING_H

# include <stdbool.h>
# include <stddef.h>
# include <stdint.h>

// TODO Must be placed in `strings.h`
void bzero(void *s, size_t n);

void *memchr(const void *s, int c, size_t n);
int memcmp(const void *s1, const void *s2, size_t n);
void* memcpy(void *dest, const void *src, size_t n);
void* memmove(void *dest, const void *src, size_t n);
void* memset(void *s, int c, size_t n);

int strcmp(const char *s1, const char *s2);
char *strcpy(char *dest, const char *src);
size_t strlen(const char *s);

#endif
