#include <debug/debug.h>
#include <elf/elf.h>

static void *inst;
static const char *func_name;

ATTR_COLD
static const char *get_symbol_name(const uint32_t offset)
{
	elf_section_header_t *section;

	if(!(section = get_section(boot_info.elf_sections, boot_info.elf_num,
		boot_info.elf_shndx, boot_info.elf_entsize, ".strtab")))
		return NULL;
	return (const char *) section->sh_addr + offset;
}

ATTR_COLD
static void get_function_symbol(elf_section_header_t *hdr, const char *name)
{
	void *ptr;
	size_t i = 0;
	elf32_sym_t *sym;

	(void) name;
	if(hdr->sh_type != SHT_SYMTAB)
		return;
	ptr = (void *) hdr->sh_addr;
	while(i < hdr->sh_size)
	{
		sym = ptr + i;
		if((uintptr_t) inst >= sym->st_value
			&& (uintptr_t) inst < sym->st_value + sym->st_size)
		{
			if(sym->st_name)
				func_name = get_symbol_name(sym->st_name);
			return;
		}
		i += sizeof(elf32_sym_t);
	}
}

ATTR_COLD
const char *get_function_name(void *i)
{
	inst = i;
	func_name = NULL;
	iterate_sections(boot_info.elf_sections, boot_info.elf_num,
		boot_info.elf_shndx, boot_info.elf_entsize, get_function_symbol);
	return func_name;
}

ATTR_COLD
void print_callstack(void *ebp, const size_t max_depth)
{
	size_t i = 0;
	void *eip = NULL;
	const char *name;

	printf("--- Callstack ---\n");
	while(ebp && i < max_depth)
	{
		if(!(eip = (void *) (*(intptr_t *) (ebp + 4))))
			break;
		if(!(name = get_function_name(eip)))
			name = "???";
		printf("%zu: %p -> %s\n", i, eip, name);
		ebp = (void *) (*(intptr_t *) ebp);
		++i;
	}
	if(ebp && eip)
		printf("...\n");
}
