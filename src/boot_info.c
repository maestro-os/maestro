#include "kernel.h"

static inline void get_boot_command(boot_info_t* info, const void* bi)
{
	info->boot_command = bi;
}

static inline void get_bootloader_name(boot_info_t* info, const void* bi)
{
	info->bootloader_name = bi;
}

static inline void get_module(boot_info_t* info, const void* bi)
{
	(void) info;
	(void) bi;

	// TODO
}

static inline void get_mem_info(boot_info_t* info, const void* bi)
{
	info->mem_lower = *((uint32_t*) bi);
	bi += sizeof(uint32_t);

	info->mem_upper = *((uint32_t*) bi);
	bi += sizeof(uint32_t);
}

static inline void get_boot_device(boot_info_t* info, const void* bi)
{
	info->biosdev = *((uint32_t*) bi);
	bi += sizeof(uint32_t);

	info->partition = *((uint32_t*) bi);
	bi += sizeof(uint32_t);

	info->sub_partition = *((uint32_t*) bi);
	bi += sizeof(uint32_t);
}

static inline void get_mem_map(boot_info_t* info, const void* bi)
{
	(void) info;
	(void) bi;

	// TODO
}

static inline void get_framebuffer_info(boot_info_t* info, const void* bi)
{
	info->framebuffer_addr = *((void**) bi);
	bi += sizeof(void*);

	info->framebuffer_pitch = *((uint32_t*) bi);
	bi += sizeof(uint32_t);

	info->framebuffer_width = *((uint32_t*) bi);
	bi += sizeof(uint32_t);

	info->framebuffer_height = *((uint32_t*) bi);
	bi += sizeof(uint32_t);

	info->framebuffer_bpp = *((uint8_t*) bi);
	bi += sizeof(uint8_t);

	info->framebuffer_type = *((uint8_t*) bi);
	bi += sizeof(uint8_t);
}

boot_info_t get_boot_info(const void* bi)
{
	boot_info_t boot_info;
	bzero(&boot_info, sizeof(boot_info));

	boot_info.total_size = *((uint32_t*) bi);
	bi += sizeof(uint32_t);
	boot_info.reserved = *((uint32_t*) bi);
	bi += sizeof(uint32_t);

	const void* end = bi - 8;

	while(bi < end) {
		const uint32_t type = *((uint32_t*) bi);
		bi += sizeof(uint32_t);
		const uint32_t size = *((uint32_t*) bi);
		bi += sizeof(uint32_t);

		switch(type) {
			case 1: {
				get_boot_command(&boot_info, bi);
				break;
			}

			case 2: {
				get_bootloader_name(&boot_info, bi);
				break;
			}

			case 3: {
				get_module(&boot_info, bi);
				break;
			}

			case 4: {
				get_mem_info(&boot_info, bi);
				break;
			}

			case 5: {
				get_boot_device(&boot_info, bi);
				break;
			}

			case 6: {
				get_mem_map(&boot_info, bi);
				break;
			}

			// TODO

			case 8: {
				get_framebuffer_info(&boot_info, bi);
				break;
			}

			// TODO
		}

		bi += size;
	}

	return boot_info;
}
