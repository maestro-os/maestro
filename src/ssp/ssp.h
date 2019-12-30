#ifndef SSP_H
# define SSP_H

# include <libc/string.h>

// TODO Use randomly generated values
# if UINTPTR_MAX == UINT32_MAX
#  define STACK_CHK_GUARD	0x994459bc
# else
#  define STACK_CHK_GUARD	0xac67b79da21dc0cd
# endif

const uintptr_t __stack_chk_guard = STACK_CHK_GUARD;

ATTR_NORETURN
void __stack_chk_fail(void);

#endif
