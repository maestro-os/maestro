#ifndef UTIL_H
# define UTIL_H

# include <libc/string.h>

# define IS_ALIGNED(ptr, n)	(((intptr_t) (ptr) & ((n) - 1)) == 0)
# define ALIGN_DOWN(ptr, n)	((void *) ((intptr_t) (ptr)\
	& ~((intptr_t) (n) - 1)))
# define ALIGN_UP(ptr, n)	(ALIGN_DOWN(ptr, n) + (n))
# define ALIGN(ptr, n)		(IS_ALIGNED((ptr), (n)) ? (ptr)\
	: ALIGN_UP((ptr), (n)))
# define SAME_PAGE(p0, p1)	(ALIGN_DOWN(p0, PAGE_SIZE)\
	== ALIGN_DOWN(p1, PAGE_SIZE))

# define CEIL_DIVISION(n0, n1)	((n0) % (n1) == 0\
	? (n0) / (n1) : (n0) / (n1) + 1)
# define POW2(n)				(((typeof(n)) 1) << (n))
# define ABS(i)		((i) < 0 ? -(i) : (i))
# define MIN(a, b)	((a) <= (b) ? (a) : (b))
# define MAX(a, b)	((a) >= (b) ? (a) : (b))

# define BIT_SIZEOF(expr)	(sizeof(expr) * 8)
# define BITFIELD_SIZE(n)	CEIL_DIVISION(n, BIT_SIZEOF(uint8_t))

# define OFFSET_OF(type, field)			((size_t) &(((type *) 0)->field))
# define CONTAINER_OF(ptr, type, field)	((void *) (ptr)\
	- OFFSET_OF(type, field))

# define VARG_COUNT(...)	(sizeof((void *[]) {__VA_ARGS__}) / sizeof(void *))

# define RB_TREE_FLAG_LEFT_LEAF		0b001
# define RB_TREE_FLAG_RIGHT_LEAF	0b010
# define RB_TREE_FLAG_RED			0b100

typedef struct rb_tree
{
	struct rb_tree *left, *right;
	char flags;
	char value[0];
} rb_tree_t;

unsigned floor_log2(const unsigned n);

int bitfield_get(const uint8_t *bitfield, size_t index);
void bitfield_set(uint8_t *bitfield, size_t index);
void bitfield_clear(uint8_t *bitfield, size_t index);
void bitfield_toggle(uint8_t *bitfield, size_t index);
void bitfield_set_range(uint8_t *bitfield, size_t begin, size_t end);
void bitfield_clear_range(uint8_t *bitfield, size_t begin, size_t end);
size_t bitfield_first_clear(const uint8_t *bitfield, size_t bitfield_size);

typedef volatile int spinlock_t;

extern void spin_lock(spinlock_t *spinlock);
extern void spin_unlock(spinlock_t *spinlock);

// TODO rb_tree functions
void rb_tree_freeall(rb_tree_t **tree);

#endif
