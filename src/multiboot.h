#ifndef MULTIBOOT_H
# define MULTIBOOT_H

# include "libc/string.h"

# define MULTIBOOT_TAG_ALIGN					8
# define MULTIBOOT_TAG_TYPE_END					0
# define MULTIBOOT_TAG_TYPE_CMDLINE				1
# define MULTIBOOT_TAG_TYPE_BOOT_LOADER_NAME	2
# define MULTIBOOT_TAG_TYPE_MODULE				3
# define MULTIBOOT_TAG_TYPE_BASIC_MEMINFO		4
# define MULTIBOOT_TAG_TYPE_BOOTDEV				5
# define MULTIBOOT_TAG_TYPE_MMAP				6
# define MULTIBOOT_TAG_TYPE_VBE					7
# define MULTIBOOT_TAG_TYPE_FRAMEBUFFER			8
# define MULTIBOOT_TAG_TYPE_ELF_SECTIONS		9
# define MULTIBOOT_TAG_TYPE_APM					10
# define MULTIBOOT_TAG_TYPE_EFI32				11
# define MULTIBOOT_TAG_TYPE_EFI64				12
# define MULTIBOOT_TAG_TYPE_SMBIOS				13
# define MULTIBOOT_TAG_TYPE_ACPI_OLD			14
# define MULTIBOOT_TAG_TYPE_ACPI_NEW			15
# define MULTIBOOT_TAG_TYPE_NETWORK				16
# define MULTIBOOT_TAG_TYPE_EFI_MMAP			17
# define MULTIBOOT_TAG_TYPE_EFI_BS				18
# define MULTIBOOT_TAG_TYPE_EFI32_IH			19
# define MULTIBOOT_TAG_TYPE_EFI64_IH			20
# define MULTIBOOT_TAG_TYPE_LOAD_BASE_ADDR		21
 
# ifndef BOOT_S

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

# endif
#endif
