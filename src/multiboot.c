#include <multiboot.h>
#include <memory/memory.h>

boot_info_t *boot_info;

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

static void handle_tag(multiboot_tag_t *tag)
{
	multiboot_tag_mmap_t *mmap_tag;
	multiboot_tag_elf_sections_t *elf_tag;

	switch(tag->type)
	{
		case MULTIBOOT_TAG_TYPE_CMDLINE:
		{
			boot_info->cmdline = ((multiboot_tag_string_t *) tag)->string;
			break;
		}

		case MULTIBOOT_TAG_TYPE_BOOT_LOADER_NAME:
		{
			boot_info->loader_name = ((multiboot_tag_string_t *) tag)->string;
			break;
		}

		case MULTIBOOT_TAG_TYPE_MODULE:
		{
			// TODO
			break;
		}

		case MULTIBOOT_TAG_TYPE_BASIC_MEMINFO:
		{
			boot_info->mem_lower
				= ((multiboot_tag_basic_meminfo_t *) tag)->mem_lower;
			boot_info->mem_upper
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
			boot_info->memory_maps_size = mmap_tag->size;
			boot_info->memory_maps_entry_size = mmap_tag->entry_size;
			boot_info->memory_maps = mmap_tag->entries;
			break;
		}

		case MULTIBOOT_TAG_TYPE_ELF_SECTIONS:
		{
			elf_tag = (multiboot_tag_elf_sections_t *) tag;
			boot_info->elf_num = elf_tag->num;
			boot_info->elf_entsize = elf_tag->entsize;
			boot_info->elf_shndx = elf_tag->shndx;
			boot_info->elf_sections = elf_tag->sections;
			break;
		}

		// TODO

		default: break;
	}
}

void read_boot_tags(void *ptr)
{
	multiboot_tag_t *tag;

	if(!ptr)
		return;
	bzero(boot_info, sizeof(boot_info_t));
	tag = ptr + 8;
	while(tag->type != MULTIBOOT_TAG_TYPE_END)
	{
		handle_tag(tag);
		tag = (multiboot_tag_t *) ((uint8_t *) tag + ((tag->size + 7) & ~7));
	}
}
