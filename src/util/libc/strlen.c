#include <stdint.h>
#include <stddef.h>

#define LOW ((size_t) -1 / 0xff)
#define HIGH (LOW * 0x80)
#define ZERO(w) (((w) - LOW) & (~(w) & HIGH))

size_t strlen(const char *s)
{
	const char *n = s;

    // Align
    for (; (uintptr_t) n % sizeof(size_t); ++n) if (!*n) return n - s;
    // Check word-by-word
    const size_t *word = (size_t *) n;
    for (; !ZERO(*word); ++word);
    n = (const char *) word;
    // Count remaining
    for (; *n; ++n)
        ;
	return n - s;
}
