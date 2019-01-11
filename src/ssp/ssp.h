#ifndef SSP_H
# define SSP_H

# include "../libc/string.h"

# if UINTPTR_MAX == UINT32_MAX
#  define STACK_CHK_GUARD	0x0
# else
#  define STACK_CHK_GUARD	0x0
# endif

const uintptr_t __stack_chk_guard = STACK_CHK_GUARD;

__attribute__((noreturn))
void __stack_chk_fail();

#endif
