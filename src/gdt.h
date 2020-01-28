#ifndef GDT_H
# define GDT_H

# include <stdint.h>

# define GDT_KERNEL_CODE_OFFSET	0x8
# define GDT_KERNEL_DATA_OFFSET	0x10
# define GDT_USER_CODE_OFFSET	0x18
# define GDT_USER_DATA_OFFSET	0x20
# define GDT_TSS_OFFSET			0x28

/*
 * Structure representing a GDT entry.
 */
typedef struct
{
	uint16_t limit_low;
	uint16_t base_low;
	uint8_t base_mid;
	uint8_t access;
	uint8_t flags_limit;
	uint8_t base_high;
} gdt_entry_t;

#endif
