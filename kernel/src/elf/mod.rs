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
pub mod relocation;

use crate::process::mem_space;
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

/// Segment flag: Execute.
pub const PF_X: u32 = 0x1;
/// Segment flag: Write.
pub const PF_W: u32 = 0x2;
/// Segment flag: Read.
pub const PF_R: u32 = 0x4;

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
/// The section holds information for dynamic linking.
pub const SHT_DYNAMIC: u32 = 0x00000006;
/// The section holds information that marks the file in some way.
pub const SHT_NOTE: u32 = 0x00000007;
/// The section is empty but contains information in its offset.
pub const SHT_NOBITS: u32 = 0x00000008;
/// The section holds relocation entries without explicit attends.
pub const SHT_REL: u32 = 0x00000009;
/// Reserved section type.
pub const SHT_SHLIB: u32 = 0x0000000a;
/// The section holds a symbol table.
pub const SHT_DYNSYM: u32 = 0x0000000b;

/// The section contains writable data.
pub const SHF_WRITE: u32 = 0x00000001;
/// The section occupies memory during execution.
pub const SHF_ALLOC: u32 = 0x00000002;
/// The section contains executable machine instructions.
pub const SHF_EXECINSTR: u32 = 0x00000004;
/// Thread-Local Storage (TLS) section.
pub const SHF_TLS: u32 = 0x400;
/// All bits included in this mask are reserved for processor-specific
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

/// No relocation.
pub const R_386_NONE: u8 = 0;
/// Relocation type.
pub const R_386_32: u8 = 1;
/// Relocation type.
pub const R_386_PC32: u8 = 2;
/// Relocation type.
pub const R_386_GOT32: u8 = 3;
/// Relocation type.
pub const R_386_PLT32: u8 = 4;
/// Relocation type.
pub const R_386_COPY: u8 = 5;
/// Relocation type.
pub const R_386_GLOB_DAT: u8 = 6;
/// Relocation type.
pub const R_386_JMP_SLOT: u8 = 7;
/// Relocation type.
pub const R_386_RELATIVE: u8 = 8;
/// Relocation type.
pub const R_386_GOTOFF: u8 = 9;
/// Relocation type.
pub const R_386_GOTPC: u8 = 10;
/// Relocation type.
pub const R_386_IRELATIVE: u8 = 42;

/// A 32 bits ELF header.
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

/// Structure representing an ELF program header.
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

impl ELF32ProgramHeader {
	/// Tells whether the program header is valid.
	///
	/// `file_size` is the size of the file.
	fn is_valid(&self, file_size: usize) -> EResult<()> {
		// TODO Check p_type
		if (self.p_offset + self.p_filesz) as usize > file_size {
			return Err(errno!(EINVAL));
		}
		if self.p_align != 0 && !self.p_align.is_power_of_two() {
			return Err(errno!(EINVAL));
		}

		Ok(())
	}

	/// Returns the flags to map the current segment into a process's memory
	/// space.
	pub fn get_mem_space_flags(&self) -> u8 {
		let mut flags = mem_space::MAPPING_FLAG_USER;
		if self.p_flags & PF_X != 0 {
			flags |= mem_space::MAPPING_FLAG_EXEC;
		}
		if self.p_flags & PF_W != 0 {
			flags |= mem_space::MAPPING_FLAG_WRITE;
		}
		flags
	}
}

/// A 32 bits ELF section header.
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

impl ELF32SectionHeader {
	/// Tells whether the section header is valid.
	///
	/// `file_size` is the size of the file.
	fn is_valid(&self, file_size: usize) -> EResult<()> {
		// TODO Check sh_name
		if self.sh_type & SHT_NOBITS == 0 && (self.sh_offset + self.sh_size) as usize > file_size {
			return Err(errno!(EINVAL));
		}
		if self.sh_addralign != 0 && !self.sh_addralign.is_power_of_two() {
			return Err(errno!(EINVAL));
		}
		Ok(())
	}
}

/// A 32 bits ELF symbol in memory.
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

impl ELF32Sym {
	/// Tells whether the symbol is defined.
	pub fn is_defined(&self) -> bool {
		self.st_shndx != 0
	}
}

/// The hash function for an ELF hash table.
pub fn hash_sym_name(name: &[u8]) -> u32 {
	let res = name.iter().fold(0u32, |mut res, c| {
		res = res.wrapping_mul(16) + *c as u32;
		res ^= res >> 24 & 0xf0;
		res
	});
	res & 0xfffffff
}
