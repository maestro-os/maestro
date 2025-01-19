// Code taken from musl. License: https://git.musl-libc.org/cgit/musl/tree/COPYRIGHT

#include <stddef.h>

int memcmp(const void *vl, const void *vr, size_t n)
{
    const unsigned char *l = vl, *r = vr;
    for (; n && *l == *r; n--, l++, r++);
    return n ? *l - *r : 0;
}
