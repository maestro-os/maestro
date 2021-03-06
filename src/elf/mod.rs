/// The Executable and Linkable Format (ELF) is a format of executable files commonly used in UNIX
/// systems. This module implements an interface to manipulate this format, including the kernel's
/// executable itself.

use core::ffi::c_void;
use core::mem::size_of;
use core::ptr::null;
use crate::memory;
use crate::util;

/// The section header is inactive.
pub const SHT_NULL: u32 = 0x00000000;
/// The section holds information defined by the program.
pub const SHT_PROGBITS: u32 = 0x00000001;
/// The section holds a symbol table.
pub const SHT_SYMTAB: u32 = 0x00000002;
/// the section holds a string table.
pub const SHT_STRTAB: u32 = 0x00000003;
/// The section holds relocation entries with explicit attends.
pub const SHT_RELA: u32 = 0x00000004;
/// The section holds a symbol hash table.
pub const SHT_HASH: u32 = 0x00000005;
/// The section holds informations for dynamic linking.
pub const SHT_DYNAMIC: u32 = 0x00000006;
/// The section holds informations that marks the file in some way.
pub const SHT_NOTE: u32 = 0x00000007;
/// The section is empty but contains information in its offset.
pub const SHT_NOBITS: u32 = 0x00000008;
/// The section holds relocation entries without explicit attends.
pub const SHT_REL: u32 = 0x00000009;
/// Reserved section type.
pub const SHT_SHLIB: u32 = 0x0000000a;
/// The section holds a symbol table.
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

/// The section contains writable data.
pub const SHF_WRITE: u32 = 0x00000001;
/// The section occupies memory during execution.
pub const SHF_ALLOC: u32 = 0x00000002;
/// The section contains executable machine instructions.
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
/// All bits included in this mask are reserved for processor-specific semantics.
pub const SHF_MASKPROC: u32 = 0xf0000000;
/// TODO doc
pub const SHF_ORDERED: u32 = 0x04000000;
/// TODO doc
pub const SHF_EXCLUDE: u32 = 0x08000000;

/// The symbol's type is not specified.
pub const STT_NOTYPE: u8 = 0;
/// The symbol is associated with a data object, such as a variable, an array, and so on.
pub const STT_OBJECT: u8 = 1;
/// The symbol is associated with a function or other executable code.
pub const STT_FUNC: u8 = 2;
/// The symbol is associated with a section.
pub const STT_SECTION: u8 = 3;
/// TODO doc
pub const STT_FILE: u8 = 4;
/// TODO doc
pub const STT_LOPROC: u8 = 13;
/// TODO doc
pub const STT_HIPROC: u8 = 15;

/// TODO doc
type ELF32Addr = u32;

/// Structure representing an ELF section header in memory.
#[derive(Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct ELF32SectionHeader {
	/// Index in the string table section specifying the name of the section.
	pub sh_name: u32,
	/// The type of the section.
	pub sh_type: u32,
	/// Section flags.
	pub sh_flags: u32,
	/// The address to the section's data in memory during execution.
	pub sh_addr: u32,
	/// The offset of the section's data in the ELF file.
	pub sh_offset: u32,
	/// The size of the section's data in bytes.
	pub sh_size: u32,
	/// Section header table index link.
	pub sh_link: u32,
	/// Extra-informations whose interpretation depends on the section type.
	pub sh_info: u32,
	/// Alignment constraints of the section in memory. `0` or `1` means that the section doesn't
	/// require specific alignment.
	pub sh_addralign: u32,
	/// If the section is a table of entry, this field holds the size of one entry. Else, holds
	/// `0`.
	pub sh_entsize: u32,
}

/// Structure representing an ELF symbol in memory.
#[derive(Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct ELF32Sym {
	/// Index in the string table section specifying the name of the symbol.
	pub st_name: u32,
	/// The value of the symbol.
	pub st_value: ELF32Addr,
	/// The size of the symbol.
	pub st_size: u32,
	/// The symbol's type and binding attributes.
	pub st_info: u8,
	/// Holds `0`.
	pub st_other: u8,
	/// The index of the section the symbol is in.
	pub st_shndx: u16,
}

/// Returns a reference to the section with name `name`. If the section is not found, returns None.
/// `sections` is a pointer to the ELF sections of the kernel in the virtual memory.
/// `sections_count` is the number of sections in the kernel.
/// `shndx` is the index of the section containing section names.
/// `entsize` is the size of section entries.
/// `name` is the name of the required section.
pub fn get_section(sections: *const c_void, sections_count: usize, shndx: usize, entsize: usize,
	name: &str) -> Option<&ELF32SectionHeader> {
	debug_assert!(sections != null::<c_void>());
	let names_section = unsafe {
		&*(sections.offset((shndx * entsize) as isize) as *const ELF32SectionHeader)
	};

	for i in 0..sections_count {
		let hdr = unsafe { // Pointer arithmetic and dereference of raw pointer
			&*(sections.offset((i * entsize) as isize) as *const ELF32SectionHeader)
		};
		let n = unsafe { // Call to unsafe function
			util::ptr_to_str(memory::kern_to_virt((names_section.sh_addr + hdr.sh_name) as _))
		};

		if n == name {
			return Some(hdr);
		}
	}

	None
}

/// Iterates over the given section headers list `sections`, calling the given closure `f` for
/// every elements with a reference and the name of the section.
/// `sections` is a pointer to the ELF sections of the kernel in the virtual memory.
/// `sections_count` is the number of sections in the kernel.
/// `shndx` is the index of the section containing section names.
/// `entsize` is the size of section entries.
/// `f` is the closure to be called for each sections.
pub fn foreach_sections<F>(sections: *const c_void, sections_count: usize, shndx: usize,
	entsize: usize, mut f: F) where F: FnMut(&ELF32SectionHeader, &str) {
	let names_section = unsafe {
		&*(sections.offset((shndx * entsize) as isize) as *const ELF32SectionHeader)
	};

	for i in 0..sections_count {
		let hdr_offset = i * size_of::<ELF32SectionHeader>();
		let hdr = unsafe { // Pointer arithmetic
			&*(sections.offset(hdr_offset as isize) as *const ELF32SectionHeader)
		};
		let n = unsafe { // Call to unsafe function
			util::ptr_to_str(memory::kern_to_virt((names_section.sh_addr + hdr.sh_name) as _))
		};
		f(hdr, n);
	}
}

/// Returns the name of the symbol at the given offset.
/// `strtab_section` is a reference to the .strtab section, containing symbol names.
/// `offset` is the offset of the symbol in the section.
/// If the offset is outside of the section, the behaviour is undefined.
pub fn get_symbol_name(strtab_section: &ELF32SectionHeader, offset: u32) -> Option<&'static str> {
	debug_assert!(offset < strtab_section.sh_size);
	Some(unsafe { // Call to unsafe function
		util::ptr_to_str(memory::kern_to_virt((strtab_section.sh_addr + offset) as _))
	})
}

/// Returns an Option containing the name of the function for the given instruction pointer. If the
/// name cannot be retrieved, the function returns None.
/// `sections` is a pointer to the ELF sections of the kernel in the virtual memory.
/// `sections_count` is the number of sections in the kernel.
/// `shndx` is the index of the section containing section names.
/// `entsize` is the size of section entries.
/// `inst` is the pointer to the instruction on the virtual memory.
/// If the section `.strtab` doesn't exist, the behaviour is undefined.
pub fn get_function_name(sections: *const c_void, sections_count: usize, shndx: usize,
	entsize: usize, inst: *const c_void) -> Option<&'static str> {
	let strtab_section = get_section(sections, sections_count, shndx, entsize, ".strtab").unwrap();
	unsafe { // TODO rm
		crate::debug::print_memory(strtab_section as *const _ as *const c_void, strtab_section.sh_size as usize);
	}

	let mut func_name: Option<&'static str> = None;
	foreach_sections(sections, sections_count, shndx, entsize,
		|hdr: &ELF32SectionHeader, _name: &str| {
			if hdr.sh_type != SHT_SYMTAB {
				return;
			}

			let ptr = memory::kern_to_virt(hdr.sh_addr as _) as *const u8;
			debug_assert!(hdr.sh_entsize > 0);

			let mut i: usize = 0;
			while i < hdr.sh_size as usize {
				let sym = unsafe { // Pointer arithmetic and dereference of raw pointer
					&*(ptr.add(i) as *const ELF32Sym)
				};

				let value = sym.st_value as usize;
				let size = sym.st_size as usize;
				if (inst as usize) >= value && (inst as usize) < (value + size) {
					if sym.st_name != 0 {
						func_name = get_symbol_name(strtab_section, sym.st_name);
					}

					break;
				}

				i += hdr.sh_entsize as usize;
			}
		});
	func_name
}
