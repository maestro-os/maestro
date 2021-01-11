#include <elf/elf.h>
#include <memory/memory.h>

/*
 * Returns the section with name `name`. If the section is not found, returns
 * NULL.
 */
elf_section_header_t *get_section(void *sections, size_t sections_count,
	size_t shndx, size_t entsize, const char *section_name)
{
	elf_section_header_t *names_section;
	size_t i = 0;
	elf_section_header_t *hdr;
	const char *n;

	if(!sections || !section_name)
		return NULL;
	names_section = sections + (shndx * entsize);
	while(i < sections_count)
	{
		hdr = sections + (i++ * sizeof(elf_section_header_t));
		n = KERN_TO_VIRT((char *) names_section->sh_addr + hdr->sh_name);
		if(strcmp(n, section_name) == 0)
			return hdr;
	}
	return NULL;
}

/*
 * Iterates over the given sections list, calling the given function with
 * section headers and names.
 */
void iterate_sections(void *sections, const size_t sections_count,
	const size_t shndx, const size_t entsize,
	void (*f)(elf_section_header_t *, const char *))
{
	elf_section_header_t *names_section;
	size_t i = 0;
	elf_section_header_t *hdr;

	if(!sections || !f)
		return;
	names_section = sections + (shndx * entsize);
	while(i < sections_count)
	{
		hdr = sections + (i++ * sizeof(elf_section_header_t));
		f(hdr, KERN_TO_VIRT((void *) names_section->sh_addr + hdr->sh_name));
	}
}
