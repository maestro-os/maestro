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

//! This module implements the ELF parser.

use super::*;
use crate::{
	elf::relocation::Relocation,
	errno,
	errno::{EResult, Errno},
	util::{bytes, bytes::AnyRepr},
};
use core::mem::size_of;

/// Returns an iterator over ELF elements.
///
/// Arguments:
/// - `table` is a slice over the elements table.
/// - `num` is the number of elements in the table.
/// - `entsize` is the size of an element in the table.
fn iter<T: AnyRepr>(
	table: &[u8],
	num: usize,
	entsize: usize,
) -> impl Iterator<Item = EResult<&T>> {
	(0..num)
		.map(move |i| {
			let begin = i * entsize;
			let end = begin + entsize;
			// Check in if in bound
			if end <= table.len() {
				bytes::from_bytes(&table[begin..end])
			} else {
				None
			}
		})
		.map(|e| e.ok_or_else(|| errno!(EINVAL)))
}

/// The ELF parser allows to parse an ELF image and retrieve information on it.
///
/// It is especially useful to load a kernel module or a userspace program.
pub struct ELFParser<'a>(&'a [u8]);

impl<'a> ELFParser<'a> {
	/// Returns the image's header.
	pub fn get_header(&self) -> &ELF32ELFHeader {
		// Safe because the image is already checked to be large enough on parser instantiation
		bytes::from_bytes(self.0).unwrap()
	}

	/// Returns the offset the content of the section containing section names.
	pub fn get_shstr_offset(&self) -> usize {
		let ehdr = self.get_header();
		let shoff = ehdr.e_shoff;
		let shentsize = ehdr.e_shentsize;

		// The offset of the section containing section names
		let shstr_off = (shoff + shentsize as u32 * ehdr.e_shstrndx as u32) as usize;

		// Will not fail because the image is already checked to be large enough at parser
		// instantiation
		let shstr = bytes::from_bytes::<ELF32SectionHeader>(&self.0[shstr_off..]).unwrap();
		shstr.sh_offset as usize
	}

	// TODO Support 64 bit
	/// Tells whether the ELF image is valid.
	fn check_image(&self) -> Result<(), Errno> {
		if self.0.len() < EI_NIDENT {
			return Err(errno!(EINVAL));
		}
		let signature = b"\x7fELF";
		if &self.0[0..signature.len()] != signature {
			return Err(errno!(EINVAL));
		}

		#[cfg(target_pointer_width = "32")]
		if self.0[EI_CLASS] != ELFCLASS32 {
			return Err(errno!(EINVAL));
		}

		#[cfg(target_endian = "big")]
		if self.0[EI_DATA] != ELFDATA2LSB {
			return Err(errno!(EINVAL));
		}

		if self.0.len() < size_of::<ELF32ELFHeader>() {
			return Err(errno!(EINVAL));
		}
		let ehdr = self.get_header();

		// TODO Check e_machine
		// TODO Check e_version

		if ehdr.e_ehsize != size_of::<ELF32ELFHeader>() as u16 {
			return Err(errno!(EINVAL));
		}

		// Check segments validity
		// Bound check is made when getting the structure from bytes
		self.try_iter_segments()
			.try_for_each(|phdr| phdr?.is_valid(self.0.len()))?;

		// Check sections validity
		if ehdr.e_shstrndx >= ehdr.e_shnum {
			return Err(errno!(EINVAL));
		}
		// Bound check is made when getting the structure from bytes
		self.try_iter_sections()
			.try_for_each(|shdr| shdr?.is_valid(self.0.len()))?;

		// TODO check relocations
		// TODO check symbols

		Ok(())
	}

	/// Creates a new instance for the given image.
	///
	/// The function checks if the image is valid. If not, the function retuns
	/// an error.
	pub fn new(image: &'a [u8]) -> EResult<Self> {
		let p = Self(image);
		p.check_image()?;
		Ok(p)
	}

	/// Returns a reference to the ELF image.
	pub fn get_image(&self) -> &[u8] {
		self.0
	}

	/// Returns an iterator on the image's segment headers.
	///
	/// If a section is out of bounds, the iterator returns an error.
	fn try_iter_segments(&self) -> impl Iterator<Item = EResult<&ELF32ProgramHeader>> {
		let ehdr = self.get_header();
		let table = &self.0[ehdr.e_phoff as usize..];
		iter(table, ehdr.e_phnum as usize, ehdr.e_phentsize as usize)
	}

	/// Returns an iterator on the image's segment headers.
	pub fn iter_segments(&self) -> impl Iterator<Item = &ELF32ProgramHeader> {
		self.try_iter_segments().filter_map(Result::ok)
	}

	/// Returns an iterator on the image's section headers.
	///
	/// If a section is out of bounds, the iterator returns an error.
	fn try_iter_sections(&self) -> impl Iterator<Item = EResult<&ELF32SectionHeader>> {
		let ehdr = self.get_header();
		let table = &self.0[ehdr.e_shoff as usize..];
		iter(table, ehdr.e_shnum as usize, ehdr.e_shentsize as usize)
	}

	/// Returns an iterator on the image's section headers.
	pub fn iter_sections(&self) -> impl Iterator<Item = &ELF32SectionHeader> {
		self.try_iter_sections().filter_map(Result::ok)
	}

	/// Returns an iterator on the relocations of the given section.
	///
	/// If the section does not have the correct type, the function returns an empty iterator.
	///
	/// If a relocation is out of bounds, the iterator returns an error.
	fn try_iter_rel<'elf, R: 'elf + AnyRepr + Relocation>(
		&'elf self,
		section: &'elf ELF32SectionHeader,
	) -> impl Iterator<Item = EResult<&'elf R>> {
		let table = &self.0[section.sh_offset as usize..];
		let mut num = (section.sh_size as usize)
			.checked_div(section.sh_entsize as usize)
			.unwrap_or(0);
		// If the section does not contain relocations, return an empty iterator
		if section.sh_type != R::REQUIRED_SECTION_TYPE {
			num = 0;
		}
		iter(table, num, section.sh_entsize as usize)
	}

	/// Returns an iterator on the section's relocations.
	///
	/// If the section doesn't have the correct type, the function returns an empty iterator.
	pub fn iter_rel<'elf, R: 'elf + AnyRepr + Relocation>(
		&'elf self,
		section: &'elf ELF32SectionHeader,
	) -> impl Iterator<Item = &'elf R> {
		self.try_iter_rel(section).filter_map(Result::ok)
	}

	/// Returns an iterator on the symbols of the given section.
	///
	/// If the section doesn't have the correct type, the function returns an empty iterator.
	///
	/// If a symbol is out of bounds, the iterator returns an error.
	pub fn try_iter_symbols(
		&self,
		section: &ELF32SectionHeader,
	) -> impl Iterator<Item = EResult<&ELF32Sym>> {
		let table = &self.0[section.sh_offset as usize..];
		let mut num = (section.sh_size as usize)
			.checked_div(section.sh_entsize as usize)
			.unwrap_or(0);
		// If the section does not contain symbols, return an empty iterator
		if section.sh_type != SHT_SYMTAB && section.sh_type != SHT_DYNSYM {
			num = 0;
		}
		iter(table, num, section.sh_entsize as usize)
	}

	/// Returns an iterator on the section's relocations.
	///
	/// If the section doesn't have the correct type, the function returns an empty iterator.
	pub fn iter_symbols(&self, section: &ELF32SectionHeader) -> impl Iterator<Item = &ELF32Sym> {
		self.try_iter_symbols(section).filter_map(Result::ok)
	}

	/// Returns the section with name `name`.
	///
	/// If the section doesn't exist, the function returns `None`.
	pub fn get_section_by_name(&self, name: &str) -> Option<&ELF32SectionHeader> {
		let shstr_off = self.get_shstr_offset();

		self.iter_sections().find(|s| {
			let section_name_begin = shstr_off + s.sh_name as usize;
			let section_name_end = section_name_begin + name.len();

			if section_name_end <= self.0.len() {
				let section_name = &self.0[section_name_begin..section_name_end];
				section_name == name.as_bytes()
			} else {
				false
			}
		})
	}

	/// Returns the symbol with name `name`.
	///
	/// If the symbol doesn't exist, the function returns `None`.
	pub fn get_symbol_by_name(&self, name: &[u8]) -> Option<&ELF32Sym> {
		// TODO use hashmap
		self.iter_sections()
			.filter_map(|section| {
				let strtab_section = self.iter_sections().nth(section.sh_link as _)?;
				Some((section, strtab_section))
			})
			.flat_map(|(section, strtab_section)| {
				self.iter_symbols(section).filter(|sym| {
					let sym_name_begin = strtab_section.sh_offset as usize + sym.st_name as usize;
					let sym_name_end = sym_name_begin + name.len();
					if sym_name_end <= self.0.len() {
						let sym_name = &self.0[sym_name_begin..sym_name_end];
						sym_name == name
					} else {
						false
					}
				})
			})
			.next()
	}

	/// Returns the name of the symbol `sym` using the string table section `strtab`.
	///
	/// If the symbol name doesn't exist, the function returns `None`.
	pub fn get_symbol_name(&self, strtab: &ELF32SectionHeader, sym: &ELF32Sym) -> Option<&[u8]> {
		if sym.st_name != 0 {
			let begin = strtab.sh_offset as usize + sym.st_name as usize;
			let max_len = strtab.sh_size as usize - sym.st_name as usize;
			let end = begin + max_len;
			let len = self.0[begin..end]
				.iter()
				.position(|b| *b == b'\0')
				.unwrap_or(max_len);
			let end = begin + len;
			Some(&self.0[begin..end])
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
		let path = &self.0[begin..end];
		// Exclude trailing `\0` if present
		let end = path.iter().position(|c| *c == b'\0').unwrap_or(path.len());
		Some(&path[..end])
	}
}
