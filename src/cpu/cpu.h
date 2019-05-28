#ifndef CPU_H
# define CPU_H

# include "../kernel.h"
# include "../libc/stdio.h"
# include "../libc/string.h"

# define MANUFACTURER_ID_LENGTH	12

extern void cpuid_init(uint8_t *highest_call, char *manufacturer);

void cpuid(void);
void cpu_reset(void);

#endif
