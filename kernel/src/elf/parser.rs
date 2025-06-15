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

//! ELF parser.

use super::*;
use crate::{
	module::relocation::Relocation,
	process::mem_space::{PROT_EXEC, PROT_READ, PROT_WRITE},
};
use core::hint::unlikely;
use utils::{bytes, limits::PAGE_SIZE};

/// The ELF's class.
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum Class {
	/// 32 bit
	Bit32,
	/// 64 bit
	#[cfg(target_pointer_width = "64")]
	Bit64,
}

impl Class {
	/// Returns the class corresponding to the given value.
	///
	/// 64 bit may be valid only if the kernel is compiled for a 64 bit target.
	///
	/// If invalid, the function returns `None`.
	#[inline]
	fn from_value(value: u8) -> Option<Self> {
		match value {
			ELFCLASS32 => Some(Class::Bit32),
			#[cfg(target_pointer_width = "64")]
			ELFCLASS64 => Some(Class::Bit64),
			_ => None,
		}
	}
}

/// Trait allowing to parse a structure from an ELF image.
pub trait Parse: Sized {
	/// Parse the structure from `data`.
	///
	/// If invalid, the function returns `None`.
	fn parse(data: &[u8], class: Class) -> Option<Self>;
}

/// Representation of a file header, bit-width-agnostic.
#[derive(Debug)]
pub struct FileHeader {
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

impl Parse for FileHeader {
	fn parse(data: &[u8], class: Class) -> Option<Self> {
		match class {
			Class::Bit32 => {
				let hdr: &ELF32ELFHeader = bytes::from_bytes(data)?;
				Some(Self {
					e_ident: hdr.e_ident,
					e_type: hdr.e_type,
					e_machine: hdr.e_machine,
					e_version: hdr.e_version,
					e_entry: hdr.e_entry as _,
					e_phoff: hdr.e_phoff as _,
					e_shoff: hdr.e_shoff as _,
					e_flags: hdr.e_flags,
					e_ehsize: hdr.e_ehsize,
					e_phentsize: hdr.e_phentsize,
					e_phnum: hdr.e_phnum,
					e_shentsize: hdr.e_shentsize,
					e_shnum: hdr.e_shnum,
					e_shstrndx: hdr.e_shstrndx,
				})
			}
			#[cfg(target_pointer_width = "64")]
			Class::Bit64 => {
				let hdr: &ELF64ELFHeader = bytes::from_bytes(data)?;
				Some(Self {
					e_ident: hdr.e_ident,
					e_type: hdr.e_type,
					e_machine: hdr.e_machine,
					e_version: hdr.e_version,
					e_entry: hdr.e_entry,
					e_phoff: hdr.e_phoff,
					e_shoff: hdr.e_shoff,
					e_flags: hdr.e_flags,
					e_ehsize: hdr.e_ehsize,
					e_phentsize: hdr.e_phentsize,
					e_phnum: hdr.e_phnum,
					e_shentsize: hdr.e_shentsize,
					e_shnum: hdr.e_shnum,
					e_shstrndx: hdr.e_shstrndx,
				})
			}
		}
	}
}

/// Representation of a program header, bit-width-agnostic.
#[derive(Debug)]
pub struct ProgramHeader {
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

impl Parse for ProgramHeader {
	fn parse(data: &[u8], class: Class) -> Option<Self> {
		match class {
			Class::Bit32 => {
				let hdr: &ELF32ProgramHeader = bytes::from_bytes(data)?;
				Some(Self {
					p_type: hdr.p_type,
					p_flags: hdr.p_flags,
					p_offset: hdr.p_offset as _,
					p_vaddr: hdr.p_vaddr as _,
					p_paddr: hdr.p_paddr as _,
					p_filesz: hdr.p_filesz as _,
					p_memsz: hdr.p_memsz as _,
					p_align: hdr.p_align as _,
				})
			}
			#[cfg(target_pointer_width = "64")]
			Class::Bit64 => {
				let hdr: &ELF64ProgramHeader = bytes::from_bytes(data)?;
				Some(Self {
					p_type: hdr.p_type,
					p_flags: hdr.p_flags,
					p_offset: hdr.p_offset,
					p_vaddr: hdr.p_vaddr,
					p_paddr: hdr.p_paddr,
					p_filesz: hdr.p_filesz,
					p_memsz: hdr.p_memsz,
					p_align: hdr.p_align,
				})
			}
		}
	}
}

impl ProgramHeader {
	/// Tells whether the program header is valid.
	///
	/// `file_size` is the size of the file.
	fn is_valid(&self, file_size: u64) -> EResult<()> {
		// TODO Check p_type
		let end = self.p_offset.checked_add(self.p_filesz as _);
		if !matches!(end, Some(end) if end <= file_size) {
			return Err(errno!(ENOEXEC));
		}
		if self.p_align > 0 {
			if !self.p_align.is_power_of_two() {
				return Err(errno!(ENOEXEC));
			}
			if self.p_offset % self.p_align != self.p_vaddr % self.p_align {
				return Err(errno!(ENOEXEC));
			}
		}
		Ok(())
	}

	/// Returns the map protection for the segment.
	pub fn mmap_prot(&self) -> u8 {
		let mut flags = 0;
		if self.p_flags & PF_X != 0 {
			flags |= PROT_EXEC;
		}
		if self.p_flags & PF_W != 0 {
			flags |= PROT_WRITE;
		}
		if self.p_flags & PF_R != 0 {
			flags |= PROT_READ;
		}
		flags
	}
}

/// Representation of a section header, bit-width-agnostic.
#[derive(Debug)]
pub struct SectionHeader {
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

impl Parse for SectionHeader {
	fn parse(data: &[u8], class: Class) -> Option<Self> {
		match class {
			Class::Bit32 => {
				let hdr: &ELF32SectionHeader = bytes::from_bytes(data)?;
				Some(Self {
					sh_name: hdr.sh_name,
					sh_type: hdr.sh_type,
					sh_flags: hdr.sh_flags as _,
					sh_addr: hdr.sh_addr as _,
					sh_offset: hdr.sh_offset as _,
					sh_size: hdr.sh_size as _,
					sh_link: hdr.sh_link,
					sh_info: hdr.sh_info,
					sh_addralign: hdr.sh_addralign as _,
					sh_entsize: hdr.sh_entsize as _,
				})
			}
			#[cfg(target_pointer_width = "64")]
			Class::Bit64 => {
				let hdr: &ELF64SectionHeader = bytes::from_bytes(data)?;
				Some(Self {
					sh_name: hdr.sh_name,
					sh_type: hdr.sh_type,
					sh_flags: hdr.sh_flags,
					sh_addr: hdr.sh_addr,
					sh_offset: hdr.sh_offset,
					sh_size: hdr.sh_size,
					sh_link: hdr.sh_link,
					sh_info: hdr.sh_info,
					sh_addralign: hdr.sh_addralign,
					sh_entsize: hdr.sh_entsize,
				})
			}
		}
	}
}

impl SectionHeader {
	/// Tells whether the section header is valid.
	///
	/// `file_size` is the size of the file.
	fn is_valid(&self, file_size: u64) -> EResult<()> {
		// TODO Check sh_name
		let end = self.sh_offset.checked_add(self.sh_size);
		if self.sh_type & SHT_NOBITS == 0 && !matches!(end, Some(end) if end <= file_size) {
			return Err(errno!(ENOEXEC));
		}
		if self.sh_addralign != 0 && !self.sh_addralign.is_power_of_two() {
			return Err(errno!(ENOEXEC));
		}
		Ok(())
	}
}

/// Representation of a symbol, bit-width-agnostic.
#[derive(Debug)]
pub struct Sym {
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

impl Parse for Sym {
	fn parse(data: &[u8], class: Class) -> Option<Self> {
		match class {
			Class::Bit32 => {
				let hdr: &ELF32Sym = bytes::from_bytes(data)?;
				Some(Self {
					st_name: hdr.st_name,
					st_info: hdr.st_info,
					st_other: hdr.st_other,
					st_shndx: hdr.st_shndx,
					st_value: hdr.st_value as _,
					st_size: hdr.st_size as _,
				})
			}
			#[cfg(target_pointer_width = "64")]
			Class::Bit64 => {
				let hdr: &ELF64Sym = bytes::from_bytes(data)?;
				Some(Self {
					st_name: hdr.st_name,
					st_info: hdr.st_info,
					st_other: hdr.st_other,
					st_shndx: hdr.st_shndx,
					st_value: hdr.st_value,
					st_size: hdr.st_size,
				})
			}
		}
	}
}

impl Sym {
	/// Tells whether the symbol is defined.
	pub fn is_defined(&self) -> bool {
		self.st_shndx != 0
	}
}

/// Representation of a relocation, bit-width-agnostic.
#[derive(Debug)]
pub struct Rel {
	/// The location of the relocation action.
	pub r_offset: u64,
	/// The relocation type and symbol index.
	pub r_info: u64,
}

impl Parse for Rel {
	fn parse(data: &[u8], class: Class) -> Option<Self> {
		match class {
			Class::Bit32 => {
				let hdr: &ELF32Rel = bytes::from_bytes(data)?;
				Some(Self {
					r_offset: hdr.r_offset as _,
					r_info: hdr.r_info as _,
				})
			}
			#[cfg(target_pointer_width = "64")]
			Class::Bit64 => {
				let hdr: &ELF64Rel = bytes::from_bytes(data)?;
				Some(Self {
					r_offset: hdr.r_offset,
					r_info: hdr.r_info,
				})
			}
		}
	}
}

impl Relocation for Rel {
	const REQUIRED_SECTION_TYPE: u32 = SHT_REL;

	fn get_offset(&self) -> usize {
		self.r_offset as _
	}

	fn get_info(&self) -> usize {
		self.r_info as _
	}
}

/// Representation of a relocation with an addend, bit-width-agnostic.
#[derive(Debug)]
pub struct Rela {
	/// The location of the relocation action.
	pub r_offset: u64,
	/// The relocation type and symbol index.
	pub r_info: u64,
	/// A constant value used to compute the relocation.
	pub r_addend: i64,
}

impl Parse for Rela {
	fn parse(data: &[u8], class: Class) -> Option<Self> {
		match class {
			Class::Bit32 => {
				let hdr: &ELF32Rela = bytes::from_bytes(data)?;
				Some(Self {
					r_offset: hdr.r_offset as _,
					r_info: hdr.r_info as _,
					r_addend: hdr.r_addend as _,
				})
			}
			#[cfg(target_pointer_width = "64")]
			Class::Bit64 => {
				let hdr: &ELF64Rela = bytes::from_bytes(data)?;
				Some(Self {
					r_offset: hdr.r_offset,
					r_info: hdr.r_info,
					r_addend: hdr.r_addend,
				})
			}
		}
	}
}

impl Relocation for Rela {
	const REQUIRED_SECTION_TYPE: u32 = SHT_RELA;

	fn get_offset(&self) -> usize {
		self.r_offset as _
	}

	fn get_info(&self) -> usize {
		self.r_info as _
	}

	fn get_addend(&self) -> isize {
		self.r_addend as _
	}
}

/// Returns an iterator over ELF elements.
///
/// Arguments:
/// - `table` is a slice over the elements table.
/// - `class` is the class to parse for.
/// - `num` is the number of elements in the table.
/// - `entsize` is the size of an element in the table.
fn iter<'elf, T: 'elf + Parse>(
	table: &'elf [u8],
	class: Class,
	num: usize,
	entsize: usize,
) -> impl Iterator<Item = EResult<T>> + use<'elf, T> {
	(0..num).map(move |i| {
		let begin = i * entsize;
		let end = begin + entsize;
		table
			.get(begin..end)
			.and_then(|data| T::parse(data, class))
			.ok_or_else(|| errno!(ENOEXEC))
	})
}

/// The ELF parser allows to parse an ELF image and retrieve information on it.
///
/// It is especially useful to load a kernel module or userspace program.
pub struct ELFParser<'elf> {
	/// The ELF data
	pub src: &'elf [u8],
	/// ELF header
	ehdr: FileHeader,
}

impl<'elf> ELFParser<'elf> {
	/// Creates a new instance for the given image.
	///
	/// The function checks if the image is valid. If not, the function returns
	/// an error.
	pub fn new(src: &'elf [u8]) -> EResult<Self> {
		if unlikely(src.len() < EI_NIDENT) {
			return Err(errno!(ENOEXEC));
		}
		// Check signature
		if unlikely(!src.starts_with(b"\x7fELF")) {
			return Err(errno!(ENOEXEC));
		}
		// Detect 32/64 bit
		let class = Class::from_value(src[EI_CLASS]).ok_or_else(|| errno!(ENOEXEC))?;
		// Check endianness
		match src[EI_DATA] {
			#[cfg(target_endian = "little")]
			ELFDATA2LSB => {}
			#[cfg(target_endian = "big")]
			ELFDATA2MSB => {}
			_ => return Err(errno!(ENOEXEC)),
		}
		// Get full header
		let ehdr = FileHeader::parse(src, class).ok_or_else(|| errno!(ENOEXEC))?;
		// Check machine type
		match ehdr.e_machine {
			#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
			// x86 | Intel 80860
			0x3 | 0x7 => {}
			#[cfg(target_arch = "x86_64")]
			// AMD x86_64
			0x3e => {}
			_ => return Err(errno!(ENOEXEC)),
		}
		// Check header validity
		let min_size = match class {
			Class::Bit32 => size_of::<ELF32ELFHeader>(),
			#[cfg(target_pointer_width = "64")]
			Class::Bit64 => size_of::<ELF64ELFHeader>(),
		};
		if unlikely((ehdr.e_ehsize as usize) < min_size) {
			return Err(errno!(ENOEXEC));
		}
		if unlikely(ehdr.e_shstrndx >= ehdr.e_shnum) {
			return Err(errno!(ENOEXEC));
		}
		Ok(Self {
			src,
			ehdr,
		})
	}

	/// Returns the image's class.
	#[inline]
	pub fn class(&self) -> Class {
		// Will not fail, because this is checked on instantiation
		Class::from_value(self.ehdr.e_ident[EI_CLASS]).unwrap()
	}

	/// Returns the image's header.
	#[inline]
	pub fn hdr(&self) -> &FileHeader {
		&self.ehdr
	}

	/// Returns an iterator on the image's segment headers.
	///
	/// If a section is out of bounds, the iterator returns an error.
	fn try_iter_segments(&self) -> impl Iterator<Item = EResult<ProgramHeader>> + use<'elf> {
		let ehdr = self.hdr();
		let table = &self.src[ehdr.e_phoff as usize..];
		iter(
			table,
			self.class(),
			ehdr.e_phnum as usize,
			ehdr.e_phentsize as usize,
		)
	}

	/// Returns an iterator on the image's segment headers.
	pub fn iter_segments(&self) -> impl Iterator<Item = ProgramHeader> + use<'elf> {
		self.try_iter_segments().filter_map(Result::ok)
	}

	/// Computes and returns the offset of the end of the loaded image, rounded to page boundary.
	pub fn get_load_size(&self) -> usize {
		self.iter_segments()
			.filter(|seg| seg.p_type == PT_LOAD)
			.map(|seg| seg.p_vaddr as usize + seg.p_memsz as usize)
			.max()
			.unwrap_or(0)
			.next_multiple_of(PAGE_SIZE)
	}

	/// Returns an iterator on the image's section headers.
	///
	/// If a section is out of bounds, the iterator returns an error.
	fn try_iter_sections(&self) -> impl Iterator<Item = EResult<SectionHeader>> + use<'elf> {
		let ehdr = self.hdr();
		let table = &self.src[ehdr.e_shoff as usize..];
		iter(
			table,
			self.class(),
			ehdr.e_shnum as usize,
			ehdr.e_shentsize as usize,
		)
	}

	/// Returns an iterator on the image's section headers.
	pub fn iter_sections(&self) -> impl Iterator<Item = SectionHeader> + use<'elf> {
		self.try_iter_sections().filter_map(Result::ok)
	}

	/// Returns the section with the given index.
	///
	/// If the section does not exist, the function returns `None`.
	pub fn get_section_by_index(&self, i: usize) -> Option<SectionHeader> {
		let hdr = self.hdr();
		// Bound check
		if i >= hdr.e_shnum as usize {
			return None;
		}
		let off = hdr.e_shoff as usize + i * hdr.e_shentsize as usize;
		let end = off + hdr.e_shentsize as usize;
		SectionHeader::parse(self.src.get(off..end)?, self.class())
	}

	/// Returns an iterator on the relocations of the given section.
	///
	/// If the section does not have the correct type, the function returns an empty iterator.
	///
	/// If a relocation is out of bounds, the iterator returns an error.
	fn try_iter_rel<R: 'elf + Parse + Relocation>(
		&self,
		section: &SectionHeader,
	) -> impl Iterator<Item = EResult<R>> + use<'elf, R> {
		let table = &self.src[section.sh_offset as usize..];
		let mut num = (section.sh_size as usize)
			.checked_div(section.sh_entsize as usize)
			.unwrap_or(0);
		// If the section does not contain relocations, return an empty iterator
		if section.sh_type != R::REQUIRED_SECTION_TYPE {
			num = 0;
		}
		iter(table, self.class(), num, section.sh_entsize as usize)
	}

	/// Returns an iterator on the section's relocations.
	///
	/// If the section doesn't have the correct type, the function returns an empty iterator.
	pub fn iter_rel<R: 'elf + Parse + Relocation>(
		&self,
		section: &SectionHeader,
	) -> impl Iterator<Item = R> + use<'elf, R> {
		self.try_iter_rel(section).filter_map(Result::ok)
	}

	/// Returns an iterator on the symbols of the given section.
	///
	/// If the section doesn't have the correct type, the function returns an empty iterator.
	///
	/// If a symbol is out of bounds, the iterator returns an error.
	pub fn try_iter_symbols(
		&self,
		section: &SectionHeader,
	) -> impl Iterator<Item = EResult<Sym>> + use<'elf> {
		let table = &self.src[section.sh_offset as usize..];
		let mut num = (section.sh_size as usize)
			.checked_div(section.sh_entsize as usize)
			.unwrap_or(0);
		// If the section does not contain symbols, return an empty iterator
		if section.sh_type != SHT_SYMTAB && section.sh_type != SHT_DYNSYM {
			num = 0;
		}
		iter(table, self.class(), num, section.sh_entsize as usize)
	}

	/// Returns an iterator on the section's relocations.
	///
	/// If the section doesn't have the correct type, the function returns an empty iterator.
	pub fn iter_symbols(&self, section: &SectionHeader) -> impl Iterator<Item = Sym> + use<'elf> {
		self.try_iter_symbols(section).filter_map(Result::ok)
	}

	/// Returns the symbol with index `i`.
	///
	/// `symtab` is the symbol table to look into.
	///
	/// If the symbol does not exist, the function returns `None`.
	pub fn get_symbol_by_index(&self, symtab: &SectionHeader, i: usize) -> Option<Sym> {
		// Bound check
		if i >= (symtab.sh_size / symtab.sh_entsize) as usize {
			return None;
		}
		let off = symtab.sh_offset as usize + i * symtab.sh_entsize as usize;
		let end = off + symtab.sh_entsize as usize;
		Sym::parse(self.src.get(off..end)?, self.class())
	}

	/// Returns the symbol with name `name`.
	///
	/// If the symbol does not exist, the function returns `None`.
	pub fn get_symbol_by_name(&self, name: &[u8]) -> Option<Sym> {
		// Fast path: get symbol from hash table
		if let Some(section) = self.get_hash_section() {
			return self.hash_find(&section, name);
		}
		// Slow path: iterate
		self.iter_sections()
			.filter_map(|section| {
				let strtab_section = self.get_section_by_index(section.sh_link as _)?;
				Some((section, strtab_section))
			})
			.flat_map(|(section, strtab_section)| {
				self.iter_symbols(&section).filter(move |sym| {
					let sym_name_begin = strtab_section.sh_offset as usize + sym.st_name as usize;
					let sym_name_end = sym_name_begin + name.len();
					let sym_name = self.src.get(sym_name_begin..sym_name_end);
					match sym_name {
						Some(sym_name) => sym_name == name,
						None => false,
					}
				})
			})
			.next()
	}

	/// Returns the name of the symbol `sym` using the string table section `strtab`.
	///
	/// If the symbol name doesn't exist, the function returns `None`.
	pub fn get_symbol_name(&self, strtab: &SectionHeader, sym: &Sym) -> Option<&[u8]> {
		if sym.st_name != 0 {
			let begin = strtab.sh_offset as usize + sym.st_name as usize;
			let max_len = strtab.sh_size as usize - sym.st_name as usize;
			let end = begin + max_len;
			let len = self.src[begin..end]
				.iter()
				.position(|b| *b == b'\0')
				.unwrap_or(max_len);
			let end = begin + len;
			Some(&self.src[begin..end])
		} else {
			None
		}
	}

	/// Returns the path to the ELF's interpreter.
	///
	/// If the ELF doesn't have an interpreter, the function returns `None`.
	pub fn get_interpreter_path(&self) -> Option<&[u8]> {
		let seg = self.iter_segments().find(|seg| seg.p_type == PT_INTERP)?;
		let begin = seg.p_offset as usize;
		let end = begin + seg.p_filesz as usize;
		// The slice won't exceed the size of the image since this is checked at parser
		// instantiation
		let path = &self.src[begin..end];
		// Exclude trailing `\0` if present
		let end = path.iter().position(|c| *c == b'\0').unwrap_or(path.len());
		Some(&path[..end])
	}

	/// Returns the section containing the hash table.
	///
	/// If the section does not exist, the function returns `None`.
	fn get_hash_section(&self) -> Option<SectionHeader> {
		self.iter_sections().find(|s| s.sh_type == SHT_HASH)
	}

	/// Finds a symbol with the given name in the hash table.
	///
	/// If the ELF does not have a hash table, if the table is invalid, or if the symbol could not
	/// be found, the function returns `None`.
	pub fn hash_find(&self, hash_section: &SectionHeader, name: &[u8]) -> Option<Sym> {
		// TODO implement SHT_GNU_HASH
		// TODO if not present, fallback to this:
		// Get required sections
		let symtab = self.get_section_by_index(hash_section.sh_link as _)?;
		let strtab = self.get_section_by_index(symtab.sh_link as _)?;
		// Get slice over hash table
		let begin = hash_section.sh_offset as usize;
		let end = begin + hash_section.sh_size as usize;
		let slice = &self.src[begin..end];
		// Closure to get a word from the slice
		let get = |off: usize| {
			let last = *slice.get(off * 4 + 3)?;
			let arr = [slice[off * 4], slice[off * 4 + 1], slice[off * 4 + 2], last];
			Some(u32::from_ne_bytes(arr))
		};
		let nbucket = get(0)? as usize;
		let nchain = get(1)? as usize;
		let hash = hash_sym_name(name) as usize;
		// Iterate, with upper bound for security
		let mut i = get(2 + hash % nbucket)? as usize;
		let mut iter = 0;
		while i != STN_UNDEF && iter < nchain + 1 {
			let sym = self.get_symbol_by_index(&symtab, i)?;
			// If the name matches, return the symbol
			if self.get_symbol_name(&strtab, &sym) == Some(name) {
				return Some(sym);
			}
			// Get next in chain
			i = get(2 + nbucket + i)? as usize;
			iter += 1;
		}
		None
	}
}
