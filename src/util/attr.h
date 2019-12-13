#ifndef ATTR_H
# define ATTR_H

# define __ATTR_PAGE_ALIGNED	__attribute__((aligned(PAGE_SIZE)))
# define __ATTR_BSS				__attribute__((section(".bss")))
# define __ATTR_RODATA			__attribute__((section(".rodata#")))
# define __ATTR_PACKED			__attribute__((packed))

#endif
