#ifndef KERNEL_H
# define KERNEL_H

# include "libc/string.h"

typedef struct boot_info
{
	uint32_t total_size;
	uint32_t reserved;

	const char* boot_command;

	const char* bootloader_name;

	uint32_t mem_lower;
	uint32_t mem_upper;

	uint32_t biosdev;
	uint32_t partition;
	uint32_t sub_partition;

	void* framebuffer_addr;
	uint32_t framebuffer_pitch;
	uint32_t framebuffer_width;
	uint32_t framebuffer_height;
	uint8_t framebuffer_bpp;
	uint8_t framebuffer_type;
} boot_info_t;

boot_info_t get_boot_info(const void* bi);

__attribute__((__noreturn__))
void abort();

#endif
