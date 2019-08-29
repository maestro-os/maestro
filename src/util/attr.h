#ifndef ATTR_H
# define ATTR_H

# include <memory/memory.h>

# define __ATTR_PAGE_ALIGNED	__attribute__((aligned(PAGE_SIZE)))
# define __ATTR_BSS				__attribute__((section("bss")))

#endif
