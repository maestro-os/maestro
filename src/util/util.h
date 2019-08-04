#ifndef UTIL_H
# define UTIL_H

# include <libc/string.h>

# define IS_ALIGNED(ptr, n)	(((intptr_t) (ptr) & ((n) - 1)) == 0)
# define ALIGN_DOWN(ptr, n)	((void *) ((intptr_t) (ptr)\
	& ~((intptr_t) (n) - 1)))
# define ALIGN_UP(ptr, n)	(ALIGN_DOWN(ptr, n) + (n))
# define ALIGN(ptr, n)		(IS_ALIGNED((ptr), (n)) ? (ptr)\
	: ALIGN_UP((ptr), (n)))

# define UPPER_DIVISION(n0, n1)	((n0) % (n1) == 0\
	? (n0) / (n1) : (n0) / (n1) + 1)
# define POW2(n)				(((typeof(n)) 1) << (n))

# define BIT_SIZEOF(expr)	(sizeof(expr) * 8)

# define OFFSET_OF(type, field)			((size_t) &(((type *) 0)->field))
# define CONTAINER_OF(ptr, type, field)	((void *) (ptr)\
	- OFFSET_OF(type, field))

unsigned floor_log2(const unsigned n);

int bitmap_get(uint8_t *bitmap, const size_t index);
void bitmap_set(uint8_t *bitmap, const size_t index);
void bitmap_clear(uint8_t *bitmap, const size_t index);
void bitmap_toggle(uint8_t *bitmap, const size_t index);
void bitmap_set_range(uint8_t *bitmap, const size_t begin, const size_t end);
void bitmap_clear_range(uint8_t *bitmap, const size_t begin, const size_t end);
size_t bitmap_first_clear(uint8_t *bitmap, const size_t bitmap_size);

typedef int spinlock_t;

extern void spin_lock(spinlock_t *spinlock);
extern void spin_unlock(spinlock_t *spinlock);
void lock(spinlock_t *spinlock);
void unlock(spinlock_t *spinlock);

#endif
