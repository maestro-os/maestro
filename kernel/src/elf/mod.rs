/*
 * Copyright 2024 Luc Lenôtre
 *
 * This file is part of Maestro.
 *
 * Maestro is free software: you can redistribute it and/or modify it under the
 * terms of the GNU General Public License as published by the Free Software
 * Foundation, either version 3 of the License, or (at your option) any later
 * version.
 *
 * Maestro is distributed in the hope that it will be useful, but WITHOUT ANY
 * WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR
 * A PARTICULAR PURPOSE. See the GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along with
 * Maestro. If not, see <https://www.gnu.org/licenses/>.
 */

//! The ELF (Executable and Linkable Format) is a format of executable files
//! commonly used in UNIX systems.
//!
//! This module implements a parser allowing to handle this format, including the kernel image
//! itself.

pub mod kernel;
pub mod parser;

use macros::AnyRepr;
use utils::{errno, errno::EResult};

/// The number of identification bytes in the ELF header.
pub const EI_NIDENT: usize = 16;

/// Identification bytes offset: File class.
pub const EI_CLASS: usize = 4;
/// Identification bytes offset: Data encoding.
pub const EI_DATA: usize = 5;
/// Identification bytes offset: Version.
pub const EI_VERSION: usize = 6;

/// File's class: Invalid class.
pub const ELFCLASSNONE: u8 = 0;
/// File's class: 32-bit objects.
pub const ELFCLASS32: u8 = 1;
/// File's class: 64-bit objects.
pub const ELFCLASS64: u8 = 2;

/// Data encoding: Invalid data encoding.
pub const ELFDATANONE: u8 = 0;
/// Data encoding: Little endian.
pub const ELFDATA2LSB: u8 = 1;
/// Data encoding: Big endian.
pub const ELFDATA2MSB: u8 = 2;

/// Object file type: No file type.
pub const ET_NONE: u16 = 0;
/// Object file type: Relocatable file.
pub const ET_REL: u16 = 1;
/// Object file type: Executable file.
pub const ET_EXEC: u16 = 2;
/// Object file type: Shared object file.
pub const ET_DYN: u16 = 3;
/// Object file type: Core file.
pub const ET_CORE: u16 = 4;
/// Object file type: Processor-specific.
pub const ET_LOPROC: u16 = 0xff00;
/// Object file type: Processor-specific.
pub const ET_HIPROC: u16 = 0xffff;

/// Required architecture: AT&T WE 32100.
pub const EM_M32: u16 = 1;
/// Required architecture: SPARC.
pub const EM_SPARC: u16 = 2;
/// Required architecture: Intel Architecture.
pub const EM_386: u16 = 3;
/// Required architecture: Motorola 68000.
pub const EM_68K: u16 = 4;
/// Required architecture: Motorola 88000.
pub const EM_88K: u16 = 5;
/// Required architecture: Intel 80860.
pub const EM_860: u16 = 7;
/// Required architecture: MIPS RS3000 Big-Endian.
pub const EM_MIPS: u16 = 8;
/// Required architecture: MIPS RS4000 Big-Endian.
pub const EM_MIPS_RS4_BE: u16 = 10;

/// Program header type: Ignored.
pub const PT_NULL: u32 = 0;
/// Program header type: Loadable segment.
pub const PT_LOAD: u32 = 1;
/// Program header type: Dynamic linking information.
pub const PT_DYNAMIC: u32 = 2;
/// Program header type: Interpreter path.
pub const PT_INTERP: u32 = 3;
/// Program header type: Auxiliary information.
pub const PT_NOTE: u32 = 4;
/// Program header type: Unspecified.
pub const PT_SHLIB: u32 = 5;
/// Program header type: The program header table itself.
pub const PT_PHDR: u32 = 6;
/// Program header type: Thread-Local Storage (TLS).
pub const PT_TLS: u32 = 7;

/// Program header type (GNU): Specifies whether the stack is executable.
pub const PT_GNU_STACK: u32 = 0x6474e551;

/// Segment flag: Execute.
pub const PF_X: u32 = 0x1;
/// Segment flag: Write.
pub const PF_W: u32 = 0x2;
/// Segment flag: Read.
pub const PF_R: u32 = 0x4;

/// The section header is inactive.
pub const SHT_NULL: u32 = 0x0;
/// The section holds information defined by the program.
pub const SHT_PROGBITS: u32 = 0x1;
/// The section holds a symbol table.
pub const SHT_SYMTAB: u32 = 0x2;
/// the section holds a string table.
pub const SHT_STRTAB: u32 = 0x3;
/// The section holds relocation entries with explicit attends.
pub const SHT_RELA: u32 = 0x4;
/// The section holds a symbol hash table.
pub const SHT_HASH: u32 = 0x5;
/// The section holds information for dynamic linking.
pub const SHT_DYNAMIC: u32 = 0x6;
/// The section holds information that marks the file in some way.
pub const SHT_NOTE: u32 = 0x7;
/// The section is empty but contains information in its offset.
pub const SHT_NOBITS: u32 = 0x8;
/// The section holds relocation entries without explicit attends.
pub const SHT_REL: u32 = 0x9;
/// Reserved section type.
pub const SHT_SHLIB: u32 = 0xa;
/// The section holds a symbol table.
pub const SHT_DYNSYM: u32 = 0xb;

/// Section flag: Contains writable data.
pub const SHF_WRITE: u32 = 0x1;
/// Section flag: Occupies memory during execution.
pub const SHF_ALLOC: u32 = 0x2;
/// Section flag: Contains executable machine instructions.
pub const SHF_EXECINSTR: u32 = 0x4;
/// Section flag: Thread-Local Storage (TLS) section.
pub const SHF_TLS: u32 = 0x400;
/// Section flag: All bits included in this mask are reserved for processor-specific
/// semantics.
pub const SHF_MASKPROC: u32 = 0xf0000000;

/// Undefined symbol index.
pub const STN_UNDEF: usize = 0;

/// The symbol's type is not specified.
pub const STT_NOTYPE: u8 = 0;
/// The symbol is associated with a data object, such as a variable, an array,
/// and so on.
pub const STT_OBJECT: u8 = 1;
/// The symbol is associated with a function or other executable code.
pub const STT_FUNC: u8 = 2;
/// The symbol is associated with a section.
pub const STT_SECTION: u8 = 3;
/// A file symbol has STB_LOCAL binding, its section index is SHN_ABS, and it
/// precedes the other STB_LOCAL symbols for the file, if it is present.
pub const STT_FILE: u8 = 4;
/// Thread-Local Storage (TLS) symbol.
pub const STT_TLS: u8 = 6;

/// 32 bit ELF header.
#[derive(AnyRepr, Clone, Debug)]
#[repr(C)]
pub struct ELF32ELFHeader {
	/// Identification bytes.
	pub e_ident: [u8; EI_NIDENT],
	/// Identifies the object file type.
	pub e_type: u16,
	/// Specifies the required machine type.
	pub e_machine: u16,
	/// The file's version.
	pub e_version: u32,
	/// The virtual address of the file's entry point.
	pub e_entry: u32,
	/// The program header table's file offset in bytes.
	pub e_phoff: u32,
	/// The section header table's file offset in bytes.
	pub e_shoff: u32,
	/// Processor-specific flags.
	pub e_flags: u32,
	/// ELF header's size in bytes.
	pub e_ehsize: u16,
	/// The size of one entry in the program header table.
	pub e_phentsize: u16,
	/// The number of entries in the program header table.
	pub e_phnum: u16,
	/// The size of one entry in the section header table.
	pub e_shentsize: u16,
	/// The number of entries in the section header table.
	pub e_shnum: u16,
	/// The section header table index holding the header of the section name
	/// string table.
	pub e_shstrndx: u16,
}

/// 64 bit ELF header.
#[cfg(target_pointer_width = "64")]
#[derive(AnyRepr, Clone, Debug)]
#[repr(C)]
pub struct ELF64ELFHeader {
	/// Identification bytes.
	pub e_ident: [u8; EI_NIDENT],
	/// Identifies the object file type.
	pub e_type: u16,
	/// Specifies the required machine type.
	pub e_machine: u16,
	/// The file's version.
	pub e_version: u32,
	/// The virtual address of the file's entry point.
	pub e_entry: u64,
	/// The program header table's file offset in bytes.
	pub e_phoff: u64,
	/// The section header table's file offset in bytes.
	pub e_shoff: u64,
	/// Processor-specific flags.
	pub e_flags: u32,
	/// ELF header's size in bytes.
	pub e_ehsize: u16,
	/// The size of one entry in the program header table.
	pub e_phentsize: u16,
	/// The number of entries in the program header table.
	pub e_phnum: u16,
	/// The size of one entry in the section header table.
	pub e_shentsize: u16,
	/// The number of entries in the section header table.
	pub e_shnum: u16,
	/// The section header table index holding the header of the section name
	/// string table.
	pub e_shstrndx: u16,
}

/// 32 bit ELF program header.
#[derive(AnyRepr, Clone, Debug)]
#[repr(C)]
pub struct ELF32ProgramHeader {
	/// Tells what kind of segment this header describes.
	pub p_type: u32,
	/// The offset of the segment's content in the file.
	pub p_offset: u32,
	/// The virtual address of the segment's content.
	pub p_vaddr: u32,
	/// The physical address of the segment's content (if relevant).
	pub p_paddr: u32,
	/// The size of the segment's content in the file.
	pub p_filesz: u32,
	/// The size of the segment's content in memory.
	pub p_memsz: u32,
	/// Segment's flags.
	pub p_flags: u32,
	/// Segment's alignment.
	pub p_align: u32,
}

/// 64 bit ELF program header.
#[cfg(target_pointer_width = "64")]
#[derive(AnyRepr, Clone, Debug)]
#[repr(C)]
pub struct ELF64ProgramHeader {
	/// Tells what kind of segment this header describes.
	pub p_type: u32,
	/// Segment's flags.
	pub p_flags: u32,
	/// The offset of the segment's content in the file.
	pub p_offset: u64,
	/// The virtual address of the segment's content.
	pub p_vaddr: u64,
	/// The physical address of the segment's content (if relevant).
	pub p_paddr: u64,
	/// The size of the segment's content in the file.
	pub p_filesz: u64,
	/// The size of the segment's content in memory.
	pub p_memsz: u64,
	/// Segment's alignment.
	pub p_align: u64,
}

/// 32 bit ELF section header.
#[derive(AnyRepr, Clone, Copy, Debug)]
#[repr(C)]
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
	/// Extra-information whose interpretation depends on the section type.
	pub sh_info: u32,
	/// Alignment constraints of the section in memory. `0` or `1` means that
	/// the section doesn't require specific alignment.
	pub sh_addralign: u32,
	/// If the section is a table of entry, this field holds the size of one
	/// entry. Else, holds `0`.
	pub sh_entsize: u32,
}

/// 64 bit ELF section header.
#[cfg(target_pointer_width = "64")]
#[derive(AnyRepr, Clone, Copy, Debug)]
#[repr(C)]
pub struct ELF64SectionHeader {
	/// Index in the string table section specifying the name of the section.
	pub sh_name: u32,
	/// The type of the section.
	pub sh_type: u32,
	/// Section flags.
	pub sh_flags: u64,
	/// The address to the section's data in memory during execution.
	pub sh_addr: u64,
	/// The offset of the section's data in the ELF file.
	pub sh_offset: u64,
	/// The size of the section's data in bytes.
	pub sh_size: u64,
	/// Section header table index link.
	pub sh_link: u32,
	/// Extra-information whose interpretation depends on the section type.
	pub sh_info: u32,
	/// Alignment constraints of the section in memory. `0` or `1` means that
	/// the section doesn't require specific alignment.
	pub sh_addralign: u64,
	/// If the section is a table of entry, this field holds the size of one
	/// entry. Else, holds `0`.
	pub sh_entsize: u64,
}

/// 32 bit ELF symbol in memory.
#[derive(AnyRepr, Clone, Copy, Debug)]
#[repr(C)]
pub struct ELF32Sym {
	/// Offset in the string table section specifying the name of the symbol.
	pub st_name: u32,
	/// The value of the symbol.
	pub st_value: u32,
	/// The size of the symbol.
	pub st_size: u32,
	/// The symbol's type and binding attributes.
	pub st_info: u8,
	/// Holds `0`.
	pub st_other: u8,
	/// The index of the section the symbol is in.
	pub st_shndx: u16,
}

/// 32 bit ELF relocation.
#[derive(AnyRepr, Clone, Copy, Debug)]
#[repr(C)]
pub struct ELF32Rel {
	/// The location of the relocation action.
	pub r_offset: u32,
	/// The relocation type and symbol index.
	pub r_info: u32,
}

/// 64 bit ELF relocation.
#[cfg(target_pointer_width = "64")]
#[derive(AnyRepr, Clone, Copy, Debug)]
#[repr(C)]
pub struct ELF64Rel {
	/// The location of the relocation action.
	pub r_offset: u64,
	/// The relocation type and symbol index.
	pub r_info: u64,
}

/// 32 bit ELF relocation with an addend.
#[derive(AnyRepr, Clone, Copy, Debug)]
#[repr(C)]
pub struct ELF32Rela {
	/// The location of the relocation action.
	pub r_offset: u32,
	/// The relocation type and symbol index.
	pub r_info: u32,
	/// A constant value used to compute the relocation.
	pub r_addend: i32,
}

/// 64 bit ELF relocation with an addend.
#[cfg(target_pointer_width = "64")]
#[derive(AnyRepr, Clone, Copy, Debug)]
#[repr(C)]
pub struct ELF64Rela {
	/// The location of the relocation action.
	pub r_offset: u64,
	/// The relocation type and symbol index.
	pub r_info: u64,
	/// A constant value used to compute the relocation.
	pub r_addend: i64,
}

/// The hash function for an ELF hash table.
pub fn hash_sym_name(name: &[u8]) -> u32 {
	let res = name.iter().fold(0u32, |mut res, c| {
		res = res.wrapping_mul(16) + *c as u32;
		res ^= (res >> 24) & 0xf0;
		res
	});
	res & 0xfffffff
}

/// 64 bit ELF symbol in memory.
#[cfg(target_pointer_width = "64")]
#[derive(AnyRepr, Clone, Copy, Debug)]
#[repr(C)]
pub struct ELF64Sym {
	/// Offset in the string table section specifying the name of the symbol.
	pub st_name: u32,
	/// The symbol's type and binding attributes.
	pub st_info: u8,
	/// Holds `0`.
	pub st_other: u8,
	/// The index of the section the symbol is in.
	pub st_shndx: u16,
	/// The value of the symbol.
	pub st_value: u64,
	/// The size of the symbol.
	pub st_size: u64,
}
