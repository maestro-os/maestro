#include <elf/elf.h>

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
		f(hdr, (void *) names_section->sh_addr + hdr->sh_name);
	}
}
