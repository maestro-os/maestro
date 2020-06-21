#ifndef UTIL_H
# define UTIL_H

# include <libc/string.h>

# if defined(KERNEL_DEBUG_SANITY) || defined(KERNEL_DEBUG_SPINLOCK)
#  include <debug/debug.h>
# endif

/*
 * Tells if pointer `ptr` is aligned on boundary `n`.
 */
# define IS_ALIGNED(ptr, n)		(((intptr_t) (ptr) & ((n) - 1)) == 0)
 /*
  * Aligns down a pointer. The retuned value shall be lower than `ptr` or equal
  * if the pointer is already aligned.
  */
# define DOWN_ALIGN(ptr, n)		((void *) ((intptr_t) (ptr)\
	& ~((intptr_t) (n) - 1)))
/*
 * Aligns up a pointer. The returned value shall be greater than `ptr`.
 */
# define UP_ALIGN(ptr, n)		(DOWN_ALIGN((ptr), (n)) + (n))
/*
 * Aligns a pointer. The returned value shall be greater than `ptr` or equal if
 * the pointer is already aligned.
 */
# define ALIGN(ptr, n)			(IS_ALIGNED((ptr), (n)) ? (ptr)\
	: UP_ALIGN((ptr), (n)))
/*
 * Tells whether `p0` and `p1` are on the same memory page or not.
 */
# define SAME_PAGE(p0, p1)		(ALIGN_DOWN((p0), PAGE_SIZE)\
	== ALIGN_DOWN((p1), PAGE_SIZE))

/*
 * Computes ceil(n0 / n1) without using floating point numbers.
 */
# define CEIL_DIVISION(n0, n1)	((n0) / (n1) + !!((n0) % (n1)))
/*
 * Computes 2^^n on unsigned integers (where `^^` is an exponent).
 */
# define POW2(n)				((typeof(n)) 1 << (n))
/*
 * Computes floor(log2(n)) without on unsigned integers.
 */
# define LOG2(n)				((n) == 0 ? 1\
	: BIT_SIZEOF(n) - __builtin_ctz(n) - 1) // TODO Check
 /*
  * Returns the absolute value for the given `i`.
  */
# define ABS(i)					((i) < 0 ? -(i) : (i))
 /*
  * Returns the lowest between `a` and `b`.
  */
# define MIN(a, b)				((a) <= (b) ? (a) : (b))
 /*
  * Returns the greatest between `a` and `b`.
  */
# define MAX(a, b)				((a) >= (b) ? (a) : (b))

/*
 * Returns the of a value in bits.
 */
# define BIT_SIZEOF(expr)	(sizeof(expr) * 8)
/*
 * Returns the size of a bitfield of `n` elements in bytes.
 */
# define BITFIELD_SIZE(n)	CEIL_DIVISION((n), BIT_SIZEOF(uint8_t))

/*
 * Returns the offset of the given field `field` in structure `type`.
 */
# define OFFSET_OF(type, field)			((size_t) &(((type *) 0)->field))
/*
 * Returns the structure of type `type` that contains the structure in field
 * `field` at pointer `ptr`.
 */
# define CONTAINER_OF(ptr, type, field)	((type *) ((void *) (ptr)\
	- OFFSET_OF(type, field)))

/*
 * Returns the number of variadic arguments for a macro.
 */
# define VARG_COUNT(...)	(sizeof((void *[]) {__VA_ARGS__}) / sizeof(void *))

/*
 * Attribute. Places the elements to the .bss section.
 */
# define ATTR_BSS			__attribute__((section(".bss")))
/*
 * Attribute. Places the elements to the .rodata section.
 */
# define ATTR_RODATA		__attribute__((section(".rodata#")))
/*
 * Attribute. Tells the compiler that the function shall not be called
 * often (allowing optimizations).
 */
# define ATTR_COLD			__attribute__((cold))
/*
 * Attribute. Tells the compiler that the function shall be called often
 * (allowing optimizations).
 */
# define ATTR_HOT			__attribute__((hot))
/*
 * Attribute. Tells the compiler that the function shall not access any values
 * in memory other than the ones passed as argument (allowing optimizations).
 */
# define ATTR_CONST			__attribute__((const))
/*
 * Attribute. Tells the compiler that the function is a malloc-like function,
 * meaning that the function allocates some memory and returns a pointer.
 * This attribute is useful because the compiler can consider that this function
 * rarely fails and then is able to optimize speculative execution.
 */
# define ATTR_MALLOC		__attribute__((malloc))
/*
 * Attribute. Tells the compiler that the function never returns.
 */
# define ATTR_NORETURN		__attribute__((noreturn))
/*
 * Attribute. Tells the compiler to pack the structure, removing padding between
 * its fields.
 */
# define ATTR_PACKED		__attribute__((packed))
/*
 * Attribute. Aligns the given element to page boundary, allowing cache
 * optimization.
 */
# define ATTR_PAGE_ALIGNED	__attribute__((aligned(PAGE_SIZE)))

/*
 * Tells the compiler that the given condition is likely to be fullfilled.
 */
# define likely(x)			__builtin_expect(!!(x), 1)
/*
 * Tells the compiler that the given condition is unlikely to be fullfilled.
 */
# define unlikely(x)		__builtin_expect(!!(x), 0)

/*
 * sanity_check(): Checks the sanity of the pointer and returns it.
 * Only enabled when compiling with the appropriate flag.
 * A pointer is considered as sane if it is in the range of the memory available
 * on the system and greater than the first megabyte or NULL.
 */
# ifdef KERNEL_DEBUG_SANITY
#  define sanity_check(x)	((typeof(x)) _debug_sanity_check(x))
# else
#  define sanity_check(x)	(x)
# endif

/*
 * Asserts the given condition. If not fullfilled, makes the kernel panic with
 * message `str`.
 */
# define assert(x, str)		if(!(x)) PANIC((str), 0)

/*
 * The type for the value inside of the avl tree structure.
 */
typedef int32_t avl_value_t; // TODO Use int64_t on 64bits

/*
 * Structure used for linked lists.
 * This structure should be used inside of other structures.
 */
typedef struct list_head
{
	/* Pointers to the previous and the next objects in the current list */
	struct list_head *prev, *next;
} list_head_t;

/*
 * Structure representing an AVL tree node.
 * This structure should be used inside of other structures.
 */
typedef struct avl_tree
{
	/* Pointers to left child, right child and parent node*/
	struct avl_tree *left, *right, *parent;
	/* Height of the current node in the tree */
	unsigned height;
	/* The value of the node used for comparison in searching */
	avl_value_t value;
} avl_tree_t;

int bitfield_get(const uint8_t *bitfield, size_t index);
void bitfield_set(uint8_t *bitfield, size_t index);
void bitfield_clear(uint8_t *bitfield, size_t index);
void bitfield_toggle(uint8_t *bitfield, size_t index);
void bitfield_set_range(uint8_t *bitfield, size_t begin, size_t end);
void bitfield_clear_range(uint8_t *bitfield, size_t begin, size_t end);
size_t bitfield_first_clear(const uint8_t *bitfield, size_t bitfield_size);

/*
 * The type for the spinlock.
 */
typedef volatile int spinlock_t;

extern void spin_lock(spinlock_t *spinlock);
extern void spin_unlock(spinlock_t *spinlock);

# ifdef KERNEL_DEBUG_SPINLOCK
#  define spin_lock(s)		debug_spin_lock(s, __FILE__, __LINE__)
#  define spin_unlock(s)	debug_spin_unlock(s, __FILE__, __LINE__)
# endif

size_t list_size(list_head_t *list);
void list_foreach(list_head_t *list, void (*f)(list_head_t *));
void list_insert_front(list_head_t **first, list_head_t *new_node);
void list_insert_before(list_head_t **first, list_head_t *node,
	list_head_t *new_node);
void list_insert_after(list_head_t **first, list_head_t *node,
	list_head_t *new_node);
void list_remove(list_head_t **first, list_head_t *node);
# ifdef KERNEL_DEBUG
int list_check(list_head_t *list);
# endif

/*
 * Generic type for a comparison function.
 */
typedef int (*cmp_func_t)(const void *, const void *);

int ptr_cmp(const void *p0, const void *p1);
int avl_val_cmp(const void *v0, const void *v1);

int avl_tree_balance_factor(const avl_tree_t *tree);
avl_tree_t *avl_tree_rotate_left(avl_tree_t *root);
avl_tree_t *avl_tree_rotate_right(avl_tree_t *root);
avl_tree_t *avl_tree_rotate_leftright(avl_tree_t *root);
avl_tree_t *avl_tree_rotate_rightleft(avl_tree_t *root);
avl_tree_t *avl_tree_search(avl_tree_t *tree, avl_value_t value, cmp_func_t f);
void avl_tree_insert(avl_tree_t **tree, avl_tree_t *node, cmp_func_t f);
void avl_tree_remove(avl_tree_t **tree, avl_tree_t *n);
void avl_tree_foreach(avl_tree_t *tree, void (*f)(avl_tree_t *));
# ifdef KERNEL_DEBUG
int avl_tree_check(avl_tree_t *tree);
void avl_tree_print(const avl_tree_t *tree);
# endif

void swap_ptr(void **p0, void **p1);

#endif
