#ifndef UTIL_H
# define UTIL_H

# include <libc/string.h>

# define IS_ALIGNED(ptr, n)	(((intptr_t) (ptr) & ((n) - 1)) == 0)
# define DOWN_ALIGN(ptr, n)	((void *) ((intptr_t) (ptr)\
	& ~((intptr_t) (n) - 1)))
# define UP_ALIGN(ptr, n)	(DOWN_ALIGN(ptr, n) + (n))
# define ALIGN(ptr, n)		(IS_ALIGNED((ptr), (n)) ? (ptr)\
	: UP_ALIGN((ptr), (n)))
# define SAME_PAGE(p0, p1)	(ALIGN_DOWN(p0, PAGE_SIZE)\
	== ALIGN_DOWN(p1, PAGE_SIZE))

# define CEIL_DIVISION(n0, n1)	((n0) / (n1) + !!((n0) % (n1)))
# define POW2(n)				(((typeof(n)) 1) << (n))
# define ABS(i)					((i) < 0 ? -(i) : (i))
# define MIN(a, b)				((a) <= (b) ? (a) : (b))
# define MAX(a, b)				((a) >= (b) ? (a) : (b))

# define BIT_SIZEOF(expr)	(sizeof(expr) * 8)
# define BITFIELD_SIZE(n)	CEIL_DIVISION(n, BIT_SIZEOF(uint8_t))

# define OFFSET_OF(type, field)			((size_t) &(((type *) 0)->field))
# define CONTAINER_OF(ptr, type, field)	((void *) (ptr)\
	- OFFSET_OF(type, field))

# define VARG_COUNT(...)	(sizeof((void *[]) {__VA_ARGS__}) / sizeof(void *))

# define ATTR_BSS			__attribute__((section(".bss")))
# define ATTR_COLD			__attribute__((cold))
# define ATTR_CONST			__attribute__((const))
# define ATTR_HOT			__attribute__((hot))
# define ATTR_MALLOC		__attribute__((malloc))
# define ATTR_NORETURN		__attribute__((noreturn))
# define ATTR_PACKED		__attribute__((packed))
# define ATTR_PAGE_ALIGNED	__attribute__((aligned(PAGE_SIZE)))
# define ATTR_RODATA		__attribute__((section(".rodata#")))

# define likely(x)			__builtin_expect(!!(x), 1)
# define unlikely(x)		__builtin_expect(!!(x), 0)

typedef struct avl_tree
{
	struct avl_tree *left, *right, *parent;
	unsigned height;
	void *value;
} avl_tree_t;

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

typedef int (*cmp_func_t)(void *, void *);

int ptr_cmp(void *p0, void *p1);

int avl_tree_balance_factor(const avl_tree_t *tree);
avl_tree_t *avl_tree_rotate_left(avl_tree_t *root);
avl_tree_t *avl_tree_rotate_right(avl_tree_t *root);
avl_tree_t *avl_tree_rotate_leftright(avl_tree_t *root);
avl_tree_t *avl_tree_rotate_rightleft(avl_tree_t *root);
avl_tree_t *avl_tree_search(avl_tree_t *tree, void *value, cmp_func_t f);
void avl_tree_insert(avl_tree_t **tree, void *value, cmp_func_t f);
void avl_tree_delete(avl_tree_t **tree, avl_tree_t *n);
void avl_tree_freeall(avl_tree_t **tree, void (*f)(void *));
# ifdef KERNEL_DEBUG
void avl_tree_print(const avl_tree_t *tree);
# endif

#endif
