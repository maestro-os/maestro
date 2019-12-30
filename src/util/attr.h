#ifndef ATTR_H
# define ATTR_H

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

#endif
