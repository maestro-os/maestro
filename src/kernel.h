#ifndef KERNEL_H
# define KERNEL_H

# include "multiboot.h"
# include "libc/string.h"

boot_info_t boot_info;

boot_info_t load_boot_info(const void* bi);

__attribute__((__noreturn__))
void abort();

#endif
