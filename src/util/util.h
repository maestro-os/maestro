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

typedef struct rb_tree
{
	struct rb_tree *left, *right;
	char color;
	uintmax_t value;
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

rb_tree_t *rb_tree_rotate_left(rb_tree_t *node);
rb_tree_t *rb_tree_rotate_right(rb_tree_t *node);
rb_tree_t *rb_tree_search(rb_tree_t *tree, uintmax_t value);
void rb_tree_insert(rb_tree_t **tree, uintmax_t value);
void rb_tree_delete(rb_tree_t **tree, uintmax_t value);
void rb_tree_freeall(rb_tree_t *tree, void (*f)(uintmax_t));

#endif
