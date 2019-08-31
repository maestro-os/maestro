#include <multiboot.h>
#include <memory/memory.h>

size_t multiboot_tags_size(void *ptr)
{
	multiboot_tag_t *tag;

	if(!ptr)
		return 0;
	tag = ptr + 8;
	while(tag->type != MULTIBOOT_TAG_TYPE_END)
		tag = (multiboot_tag_t *) ((uint8_t *) tag + ((tag->size + 7) & ~7));
	tag = (multiboot_tag_t *) ((uint8_t *) tag + ((tag->size + 7) & ~7));
	return (size_t) ((void *) tag - ptr);
}

static void handle_tag(multiboot_tag_t *tag, boot_info_t *info)
{
	multiboot_tag_mmap_t *mmap_tag;

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
			mmap_tag = (multiboot_tag_mmap_t *) tag;
			info->memory_maps_size = mmap_tag->size;
			info->memory_maps_entry_size = mmap_tag->entry_size;
			info->memory_maps = mmap_tag->entries;
			break;
		}

		case MULTIBOOT_TAG_TYPE_ELF_SECTIONS:
		{
			info->elf_sections = (multiboot_tag_elf_sections_t *) tag;
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
