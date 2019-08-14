#include <multiboot.h>
#include <memory/memory.h>

static void handle_tag(multiboot_tag_t *tag, boot_info_t *info)
{
	multiboot_tag_mmap_t *t;

	switch(tag->type)
	{
		case MULTIBOOT_TAG_TYPE_CMDLINE:
		{
			info->cmdline = ((multiboot_tag_string_t *) tag)->string;
			break;
		}

		case MULTIBOOT_TAG_TYPE_BOOT_LOADER_NAME:
		{
			info->loader_name = ((multiboot_tag_string_t *) tag)->string;
			break;
		}

		case MULTIBOOT_TAG_TYPE_MODULE:
		{
			// TODO
			break;
		}

		case MULTIBOOT_TAG_TYPE_BASIC_MEMINFO:
		{
			info->mem_lower
				= ((multiboot_tag_basic_meminfo_t *) tag)->mem_lower;
			info->mem_upper
				= ((multiboot_tag_basic_meminfo_t *) tag)->mem_upper;
			break;
		}

		case MULTIBOOT_TAG_TYPE_BOOTDEV:
		{
			// TODO
			break;
		}

		case MULTIBOOT_TAG_TYPE_MMAP:
		{
			t = (multiboot_tag_mmap_t *) tag;
			info->memory_maps_size = t->size;
			info->memory_maps_entry_size = t->entry_size;
			info->memory_maps = t->entries;
			break;
		}

		// TODO

		default: break;
	}
}

void read_boot_tags(void *ptr, boot_info_t *info)
{
	multiboot_tag_t *tag;

	if(!ptr || !info)
		return;
	bzero(info, sizeof(info));
	tag = ptr + 8;
	while(tag->type != MULTIBOOT_TAG_TYPE_END)
	{
		handle_tag(tag, info);
		tag = (multiboot_tag_t *) ((uint8_t *) tag + ((tag->size + 7) & ~7));
	}
}
