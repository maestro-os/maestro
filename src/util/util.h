#ifndef UTIL_H
# define UTIL_H

# include "../libc/string.h"

# define IS_ALIGNED(ptr, n)	(((intptr_t) (ptr) & ((n) - 1)) == 0)
# define ALIGN_DOWN(ptr, n)	((void *) ((intptr_t) (ptr)\
	& ~((intptr_t) (n) - 1)))
# define ALIGN_UP(ptr, n)	(ALIGN_DOWN(ptr, n) + (n))
# define ALIGN(ptr, n)		(IS_ALIGNED((ptr), (n)) ? (ptr)\
	: ALIGN_UP((ptr), (n)))

# define UPPER_DIVISION(n0, n1)	((n0) % (n1) == 0\
	? (n0) / (n1) : (n0) / (n1) + 1)
# define POW2(n)				(1 << (n))

# define BIT_SIZEOF(expr)	(sizeof(expr) * 8)

# define OFFSET_OF(type, field)			((size_t) &(((type *) 0)->field))
# define CONTAINER_OF(ptr, type, field)	((void *) (ptr)\
	- OFFSET_OF(type, field))

inline unsigned log2(const unsigned n)
{
	unsigned r = 0;
	while((unsigned) POW2(r) < n) ++r;

	return r;
}

int bitmap_get(char *bitmap, const size_t index);
void bitmap_set(char *bitmap, const size_t index);
void bitmap_clear(char *bitmap, const size_t index);
void bitmap_toggle(char *bitmap, const size_t index);
void bitmap_set_range(char *bitmap, const size_t begin, const size_t end);
void bitmap_clear_range(char *bitmap, const size_t begin, const size_t end);

typedef uint8_t spinlock_t;

extern void spin_lock(spinlock_t *spinlock);
extern void spin_unlock(spinlock_t *spinlock);

#endif
