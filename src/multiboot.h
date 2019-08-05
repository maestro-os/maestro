#ifndef MULTIBOOT_H
# define MULTIBOOT_H

# include <libc/string.h>

# define MULTIBOOT2_BOOTLOADER_MAGIC			0x36d76289
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

# define MULTIBOOT_MEMORY_AVAILABLE				1
# define MULTIBOOT_MEMORY_RESERVED				2
# define MULTIBOOT_MEMORY_ACPI_RECLAIMABLE		3
# define MULTIBOOT_MEMORY_NVS					4
# define MULTIBOOT_MEMORY_BADRAM				5

# define MULTIBOOT_FRAMEBUFFER_TYPE_INDEXED		0
# define MULTIBOOT_FRAMEBUFFER_TYPE_RGB			1
# define MULTIBOOT_FRAMEBUFFER_TYPE_EGA_TEXT	2
 
typedef struct multiboot_header_tag
{
	uint16_t type;
	uint16_t flags;
	uint32_t size;
} multiboot_header_tag_t;

typedef struct multiboot_header_tag_information_request
{
	uint16_t type;
	uint16_t flags;
	uint32_t size;
	uint32_t *requests;
} multiboot_header_tag_information_request_t;

typedef struct multiboot_header_tag_address
{
	uint16_t type;
	uint16_t flags;
	uint32_t size;
	uint32_t header_addr;
	uint32_t load_addr;
	uint32_t load_end_addr;
	uint32_t bss_end_addr;
} multiboot_header_tag_address_t;

typedef struct multiboot_header_tag_entry_address
{
	uint16_t type;
	uint16_t flags;
	uint32_t size;
	uint32_t entry_addr;
} multiboot_header_tag_entry_address_t;

typedef struct multiboot_header_tag_console_flags
{
	uint16_t type;
	uint16_t flags;
	uint32_t size;
	uint32_t console_flags;
} multiboot_header_tag_console_flags_t;

typedef struct multiboot_header_tag_framebuffer
{
	uint16_t type;
	uint16_t flags;
	uint32_t size;
	uint32_t width;
	uint32_t height;
	uint32_t depth;
} multiboot_header_tag_framebuffer_t;

typedef struct multiboot_header_tag_module_align
{
	uint16_t type;
	uint16_t flags;
	uint32_t size;
} multiboot_header_tag_module_align_t;

typedef struct multiboot_header_tag_relocatable
{
	uint16_t type;
	uint16_t flags;
	uint32_t size;
	uint32_t min_addr;
	uint32_t max_addr;
	uint32_t align;
	uint32_t preference;
} multiboot_header_tag_relocatable_t;

typedef struct multiboot_color
{
	uint8_t red;
	uint8_t green;
	uint8_t blue;
} multiboot_color_t;

typedef struct multiboot_mmap_entry
{
	uint64_t addr;
	uint64_t len;
	uint32_t type;
	uint32_t zero;
} multiboot_mmap_entry_t;

typedef struct multiboot_tag
{
	uint32_t type;
	uint32_t size;
} multiboot_tag_t;

typedef struct multiboot_tag_string
{
	uint32_t type;
	uint32_t size;
	char *string;
} multiboot_tag_string_t;

typedef struct multiboot_tag_module
{
	uint32_t type;
	uint32_t size;
	uint32_t mod_start;
	uint32_t mod_end;
	char *cmdline;
} multiboot_tag_module_t;

typedef struct multiboot_tag_basic_meminfo
{
	uint32_t type;
	uint32_t size;
	uint32_t mem_lower;
	uint32_t mem_upper;
} multiboot_tag_basic_meminfo_t;

typedef struct multiboot_tag_bootdev
{
	uint32_t type;
	uint32_t size;
	uint32_t biosdev;
	uint32_t slice;
	uint32_t part;
} multiboot_tag_bootdev_t;

typedef struct multiboot_tag_mmap
{
	uint32_t type;
	uint32_t size;
	uint32_t entry_size;
	uint32_t entry_version;
	struct multiboot_mmap_entry *entries;
} multiboot_tag_mmap_t;

typedef struct multiboot_vbe_info_block
{
	uint8_t external_specification[512];
} multiboot_vbe_info_block_t;

typedef struct multiboot_vbe_mode_info_block
{
	uint8_t external_specification[256];
} multiboot_vbe_mode_info_block_t;

typedef struct multiboot_tag_vbe
{
	uint32_t type;
	uint32_t size;

	uint16_t vbe_mode;
	uint16_t vbe_interface_seg;
	uint16_t vbe_interface_off;
	uint16_t vbe_interface_len;

	struct multiboot_vbe_info_block vbe_control_info;
	struct multiboot_vbe_mode_info_block vbe_mode_info;
} multiboot_tag_vbe_t;

typedef struct multiboot_tag_framebuffer_common
{
	uint32_t type;
	uint32_t size;

	uint64_t framebuffer_addr;
	uint32_t framebuffer_pitch;
	uint32_t framebuffer_width;
	uint32_t framebuffer_height;
	uint8_t framebuffer_bpp;
	uint8_t framebuffer_type;
	uint16_t reserved;
} multiboot_tag_framebuffer_common_t;

typedef struct multiboot_tag_framebuffer
{
	struct multiboot_tag_framebuffer_common common;

	union
	{
		struct
		{
			uint16_t framebuffer_palette_num_colors;
			struct multiboot_color *framebuffer_palette;
		};

		struct
		{
			uint8_t framebuffer_red_field_position;
			uint8_t framebuffer_red_mask_size;
			uint8_t framebuffer_green_field_position;
			uint8_t framebuffer_green_mask_size;
			uint8_t framebuffer_blue_field_position;
			uint8_t framebuffer_blue_mask_size;
		};
	};
} multiboot_tag_framebuffer_t;

typedef struct multiboot_tag_elf_sections
{
	uint32_t type;
	uint32_t size;
	uint32_t num;
	uint32_t entsize;
	uint32_t shndx;
	char *sections;
} multiboot_tag_elf_sections_t;

typedef struct multiboot_tag_apm
{
	uint32_t type;
	uint32_t size;
	uint16_t version;
	uint16_t cseg;
	uint32_t offset;
	uint16_t cseg_16;
	uint16_t dseg;
	uint16_t flags;
	uint16_t cseg_len;
	uint16_t cseg_16_len;
	uint16_t dseg_len;
} multiboot_tag_apm_t;

typedef struct multiboot_tag_efi32
{
	uint32_t type;
	uint32_t size;
	uint32_t pointer;
} multiboot_tag_efi32_t;

typedef struct multiboot_tag_efi64
{
	uint32_t type;
	uint32_t size;
	uint64_t pointer;
} multiboot_tag_efi64_t;

typedef struct multiboot_tag_smbios
{
	uint32_t type;
	uint32_t size;
	uint8_t major;
	uint8_t minor;
	uint8_t reserved[6];
	uint8_t *tables;
} multiboot_tag_smbios_t;

typedef struct multiboot_tag_old_acpi
{
	uint32_t type;
	uint32_t size;
	uint8_t *rsdp;
} multiboot_tag_old_acpi_t;

typedef struct multiboot_tag_new_acpi
{
	uint32_t type;
	uint32_t size;
	uint8_t *rsdp;
} multiboot_tag_new_acpi_t;

typedef struct multiboot_tag_network
{
	uint32_t type;
	uint32_t size;
	uint8_t *dhcpack;
} multiboot_tag_network_t;

typedef struct multiboot_tag_efi_mmap
{
	uint32_t type;
	uint32_t size;
	uint32_t descr_size;
	uint32_t descr_vers;
	uint8_t *efi_mmap;
} multiboot_tag_efi_mmap_t;

typedef struct multiboot_tag_efi32_ih
{
	uint32_t type;
	uint32_t size;
	uint32_t pointer;
} multiboot_tag_efi32_ih_t;

typedef struct multiboot_tag_efi64_ih
{
	uint32_t type;
	uint32_t size;
	uint64_t pointer;
} multiboot_tag_efi64_ih_t;

typedef struct multiboot_tag_load_base_addr
{
	uint32_t type;
	uint32_t size;
	uint32_t load_base_addr;
} multiboot_tag_load_base_addr_t;

typedef struct boot_info_t
{
	const char *cmdline;
	const char *loader_name;

	uint32_t mem_lower;
	uint32_t mem_upper;
	size_t memory_maps_count;
	multiboot_mmap_entry_t *memory_maps;

	// TODO
} boot_info_t;

void read_boot_tags(void *ptr, boot_info_t *info);

#endif
