#ifndef ELF_H
# define ELF_H

# include <kernel.h>

# define SHT_NULL			0x00000000
# define SHT_PROGBITS		0x00000001
# define SHT_SYMTAB			0x00000002
# define SHT_STRTAB			0x00000003
# define SHT_RELA			0x00000004
# define SHT_HASH			0x00000005
# define SHT_DYNAMIC		0x00000006
# define SHT_NOTE			0x00000007
# define SHT_NOBITS			0x00000008
# define SHT_REL			0x00000009
# define SHT_SHLIB			0x0000000a
# define SHT_DYNSYM			0x0000000b
# define SHT_INIT_ARRAY		0x0000000e
# define SHT_FINI_ARRAY		0x0000000f
# define SHT_PREINIT_ARRAY	0x00000010
# define SHT_GROUP			0x00000011
# define SHT_SYMTAB_SHNDX	0x00000012
# define SHT_NUM			0x00000013
# define SHT_LOOS			0x60000000

# define SHF_WRITE				0x00000001
# define SHF_ALLOC				0x00000002
# define SHF_EXECINSTR			0x00000004
# define SHF_MERGE				0x00000010
# define SHF_STRINGS			0x00000020
# define SHF_INFO_LINK			0x00000040
# define SHF_LINK_ORDER			0x00000080
# define SHF_OS_NONCONFORMING	0x00000100
# define SHF_GROUP				0x00000200
# define SHF_TLS				0x00000400
# define SHF_MASKOS				0x0ff00000
# define SHF_MASKPROC			0xf0000000
# define SHF_ORDERED			0x04000000
# define SHF_EXCLUDE			0x08000000

__attribute__((packed))
struct elf_section_header
{
	uint32_t sh_name;
	uint32_t sh_type;
	uint32_t sh_flags;
	uint32_t sh_addr;
	uint32_t sh_offset;
	uint32_t sh_size;
	uint32_t sh_link;
	uint32_t sh_info;
	uint32_t sh_addralign;
	uint32_t sh_entsize;
};

typedef struct elf_section_header elf_section_header_t;

void iterate_sections(void *sections, size_t sections_count,
	size_t shndx, size_t entsize,
		void (*f)(elf_section_header_t *, const char *));

#endif
