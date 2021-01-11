/// TODO doc

use core::ffi::c_void;
use crate::memory::NULL;
use crate::util;

/// TODO doc
pub const SHT_NULL: u32 = 0x00000000;
/// TODO doc
pub const SHT_PROGBITS: u32 = 0x00000001;
/// TODO doc
pub const SHT_SYMTAB: u32 = 0x00000002;
/// TODO doc
pub const SHT_STRTAB: u32 = 0x00000003;
/// TODO doc
pub const SHT_RELA: u32 = 0x00000004;
/// TODO doc
pub const SHT_HASH: u32 = 0x00000005;
/// TODO doc
pub const SHT_DYNAMIC: u32 = 0x00000006;
/// TODO doc
pub const SHT_NOTE: u32 = 0x00000007;
/// TODO doc
pub const SHT_NOBITS: u32 = 0x00000008;
/// TODO doc
pub const SHT_REL: u32 = 0x00000009;
/// TODO doc
pub const SHT_SHLIB: u32 = 0x0000000a;
/// TODO doc
pub const SHT_DYNSYM: u32 = 0x0000000b;
/// TODO doc
pub const SHT_INIT_ARRAY: u32 = 0x0000000e;
/// TODO doc
pub const SHT_FINI_ARRAY: u32 = 0x0000000f;
/// TODO doc
pub const SHT_PREINIT_ARRAY: u32 = 0x00000010;
/// TODO doc
pub const SHT_GROUP: u32 = 0x00000011;
/// TODO doc
pub const SHT_SYMTAB_SHNDX: u32 = 0x00000012;
/// TODO doc
pub const SHT_NUM: u32 = 0x00000013;
/// TODO doc
pub const SHT_LOOS: u32 = 0x60000000;

/// TODO doc
pub const SHF_WRITE: u32 = 0x00000001;
/// TODO doc
pub const SHF_ALLOC: u32 = 0x00000002;
/// TODO doc
pub const SHF_EXECINSTR: u32 = 0x00000004;
/// TODO doc
pub const SHF_MERGE: u32 = 0x00000010;
/// TODO doc
pub const SHF_STRINGS: u32 = 0x00000020;
/// TODO doc
pub const SHF_INFO_LINK: u32 = 0x00000040;
/// TODO doc
pub const SHF_LINK_ORDER: u32 = 0x00000080;
/// TODO doc
pub const SHF_OS_NONCONFORMING: u32 = 0x00000100;
/// TODO doc
pub const SHF_GROUP: u32 = 0x00000200;
/// TODO doc
pub const SHF_TLS: u32 = 0x00000400;
/// TODO doc
pub const SHF_MASKOS: u32 = 0x0ff00000;
/// TODO doc
pub const SHF_MASKPROC: u32 = 0xf0000000;
/// TODO doc
pub const SHF_ORDERED: u32 = 0x04000000;
/// TODO doc
pub const SHF_EXCLUDE: u32 = 0x08000000;

/// TODO doc
pub const ELF32_STT_NOTYPE: u8 = 0;
/// TODO doc
pub const ELF32_STT_OBJECT: u8 = 1;
/// TODO doc
pub const ELF32_STT_FUNC: u8 = 2;
/// TODO doc
pub const ELF32_STT_SECTION: u8 = 3;
/// TODO doc
pub const ELF32_STT_FILE: u8 = 4;
/// TODO doc
pub const ELF32_STT_LOPROC: u8 = 13;
/// TODO doc
pub const ELF32_STT_HIPROC: u8 = 15;

/// TODO doc
type ELF32Addr = u32;

/// Structure representing an ELF section header in memory.
#[repr(C, packed)]
pub struct ELF32SectionHeader {
	pub sh_name: u32,
	pub sh_type: u32,
	pub sh_flags: u32,
	pub sh_addr: u32,
	pub sh_offset: u32,
	pub sh_size: u32,
	pub sh_link: u32,
	pub sh_info: u32,
	pub sh_addralign: u32,
	pub sh_entsize: u32,
}

/// Structure representing an ELF symbol in memory.
#[repr(C, packed)]
pub struct ELF32Sym {
	pub st_name: u32,
	pub st_value: ELF32Addr,
	pub st_size: u32,
	pub st_info: u8,
	pub st_other: u8,
	pub st_shndx: u16,
}

/// Returns a reference to the section with name `name`. If the section is not found, returns None.
/// TODO document every arguments
pub fn get_section(sections: *const c_void, sections_count: usize, shndx: usize, entsize: usize,
	name: &str) -> Option<&ELF32SectionHeader> {
	debug_assert!(sections != NULL);

	let names_section = unsafe {
		&*(sections.offset((shndx * entsize) as isize) as *const ELF32SectionHeader)
	};
	let mut i = 0;
	while i < sections_count {
		let hdr = unsafe {
			&*(sections.offset((i * core::mem::size_of::<ELF32SectionHeader>()) as isize)
				as *const ELF32SectionHeader)
		};
		let n = unsafe {
			util::ptr_to_str(mem_alloc::kern_to_virt((names_section.sh_addr + hdr.sh_name) as _))
		};
		if n == name {
			return Some(hdr);
		}
		i += 1;
	}
	None
}

/// Iterates over the given section headers list `sections`, calling the given closure `f` for every
/// elements with a reference and the name of the section.
/// TODO document every arguments
pub fn foreach_sections<T>(sections: *const c_void, sections_count: usize, shndx: usize,
	entsize: usize, mut f: T) where T: FnMut(&ELF32SectionHeader, &str) {
	let names_section = unsafe {
		&*(sections.offset((shndx * entsize) as isize) as *const ELF32SectionHeader)
	};
	let mut i = 0;
	while i < sections_count {
		let hdr = unsafe {
			&*(sections.offset((i * core::mem::size_of::<ELF32SectionHeader>()) as isize) as *const ELF32SectionHeader)
		};
		let n = unsafe {
			util::ptr_to_str(mem_alloc::kern_to_virt((names_section.sh_addr + hdr.sh_name) as _))
		};
		f(hdr, n);
		i += 1;
	}
}
