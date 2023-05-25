//! This module implements the ELF parser.

use super::iter::ELFIterator;
use super::*;
use crate::elf::relocation::ELF32Rel;
use crate::elf::relocation::ELF32Rela;
use crate::errno;
use crate::errno::Errno;
use core::mem::size_of;

/// The ELF parser allows to parse an ELF image and retrieve informations on it.
///
/// It is especially useful to load a kernel module or a userspace program.
pub struct ELFParser<'a> {
	/// The ELF image.
	image: &'a [u8],
}

impl<'a> ELFParser<'a> {
	/// Returns the image's header.
	pub fn get_header(&self) -> &ELF32ELFHeader {
		// Safe because the image is already checked to be large enough on parser instanciation
		unsafe { util::reinterpret::<ELF32ELFHeader>(self.image) }.unwrap()
	}

	/// Returns the offset the content of the section containing section names.
	pub fn get_shstr_offset(&self) -> usize {
		let ehdr = self.get_header();
		let shoff = ehdr.e_shoff;
		let shentsize = ehdr.e_shentsize;

		// The offset of the section containing section names
		let shstr_off = (shoff + shentsize as u32 * ehdr.e_shstrndx as u32) as usize;

		// Safe because the image is already checked to be large enough on parser instanciation
		let shstr =
			unsafe { util::reinterpret::<ELF32SectionHeader>(&self.image[shstr_off..]) }.unwrap();
		shstr.sh_offset as usize
	}

	// TODO Support 64 bit
	/// Tells whether the ELF image is valid.
	fn check_image(&self) -> Result<(), Errno> {
		let signature = [0x7f, b'E', b'L', b'F'];

		if self.image.len() < EI_NIDENT {
			return Err(errno!(EINVAL));
		}
		if self.image[0..signature.len()] != signature {
			return Err(errno!(EINVAL));
		}

		// TODO Check relative to current architecture
		if self.image[EI_CLASS] != ELFCLASS32 {
			return Err(errno!(EINVAL));
		}

		// TODO Check relative to current architecture
		if self.image[EI_DATA] != ELFDATA2LSB {
			return Err(errno!(EINVAL));
		}

		if self.image.len() < size_of::<ELF32ELFHeader>() {
			return Err(errno!(EINVAL));
		}
		let ehdr = self.get_header();

		// TODO Check e_machine
		// TODO Check e_version

		if ehdr.e_ehsize != size_of::<ELF32ELFHeader>() as u16 {
			return Err(errno!(EINVAL));
		}

		let phdr_end = ehdr.e_phoff as usize + ehdr.e_phentsize as usize * ehdr.e_phnum as usize;
		if phdr_end > self.image.len() {
			return Err(errno!(EINVAL));
		}

		let shdr_end = ehdr.e_shoff as usize + ehdr.e_shentsize as usize * ehdr.e_shnum as usize;
		if shdr_end > self.image.len() {
			return Err(errno!(EINVAL));
		}
		if ehdr.e_shstrndx >= ehdr.e_shnum {
			return Err(errno!(EINVAL));
		}

		for i in 0..ehdr.e_phnum {
			let off = (ehdr.e_phoff + ehdr.e_phentsize as u32 * i as u32) as usize;
			let phdr = unsafe {
				// Safe because in range of the slice
				util::reinterpret::<ELF32ProgramHeader>(&self.image[off..])
			}
			.unwrap();

			phdr.is_valid(self.image.len())?;
		}

		for i in 0..ehdr.e_shnum {
			let off = (ehdr.e_shoff + ehdr.e_shentsize as u32 * i as u32) as usize;
			let shdr = unsafe {
				// Safe because in range of the slice
				util::reinterpret::<ELF32SectionHeader>(&self.image[off..])
			}
			.unwrap();

			shdr.is_valid(self.image.len())?;
		}

		Ok(())
	}

	/// Creates a new instance for the given image.
	///
	/// The function checks if the image is valid. If not, the function retuns
	/// an error.
	pub fn new(image: &'a [u8]) -> Result<Self, Errno> {
		let p = Self {
			image,
		};

		p.check_image()?;
		Ok(p)
	}

	/// Returns a reference to the ELF image.
	pub fn get_image(&self) -> &[u8] {
		self.image
	}

	/// Returns an iterator on the image's segment headers.
	pub fn iter_segments(&self) -> ELFIterator<ELF32ProgramHeader> {
		let ehdr = self.get_header();
		let phoff = ehdr.e_phoff as usize;
		let phnum = ehdr.e_phnum as usize;
		let phentsize = ehdr.e_phentsize as usize;

		let end = phoff + (phnum * phentsize);
		let table = &self.image[phoff..end];

		ELFIterator::<ELF32ProgramHeader>::new(table, phentsize)
	}

	/// Returns an iterator on the image's section headers.
	pub fn iter_sections(&self) -> ELFIterator<ELF32SectionHeader> {
		let ehdr = self.get_header();
		let shoff = ehdr.e_shoff as usize;
		let shnum = ehdr.e_shnum as usize;
		let shentsize = ehdr.e_shentsize as usize;

		let end = shoff + (shnum * shentsize);
		let table = &self.image[shoff..end];

		ELFIterator::<ELF32SectionHeader>::new(table, shentsize)
	}

	// FIXME: Passing an invalid section is undefined
	/// Returns an iterator on the relocations (without addend) of the given section.
	///
	/// If the section doesn't have the correct type, the function returns an empty iterator.
	pub fn iter_rel(&self, section: &ELF32SectionHeader) -> ELFIterator<ELF32Rel> {
		let begin = section.sh_offset as usize;
		let mut end = begin + section.sh_size as usize;
		if section.sh_type != SHT_REL {
			end = begin;
		}

		let table = &self.image[begin..end];
		ELFIterator::<ELF32Rel>::new(table, section.sh_entsize as usize)
	}

	/// Returns an iterator on the relocations (with addend) of the given section.
	///
	/// If the section doesn't have the correct type, the function returns an empty iterator.
	pub fn iter_rela(&self, section: &ELF32SectionHeader) -> ELFIterator<ELF32Rela> {
		let begin = section.sh_offset as usize;
		let mut end = begin + section.sh_size as usize;
		if section.sh_type != SHT_RELA {
			end = begin;
		}

		let table = &self.image[begin..end];
		ELFIterator::<ELF32Rela>::new(table, section.sh_entsize as usize)
	}

	/// Returns an iterator on the symbols of the given section.
	///
	/// If the section doesn't have the correct type, the function returns an empty iterator.
	pub fn iter_symbols(&self, section: &ELF32SectionHeader) -> ELFIterator<ELF32Sym> {
		let begin = section.sh_offset as usize;
		let mut end = begin + section.sh_size as usize;
		if section.sh_type != SHT_SYMTAB && section.sh_type != SHT_DYNSYM {
			end = begin;
		}

		let table = &self.image[begin..end];
		ELFIterator::<ELF32Sym>::new(table, section.sh_entsize as usize)
	}

	/// Returns the section with name `name`.
	///
	/// If the section doesn't exist, the function returns `None`.
	pub fn get_section_by_name(&self, name: &str) -> Option<&ELF32SectionHeader> {
		let shstr_off = self.get_shstr_offset();

		self.iter_sections().find(|s| {
			let section_name_begin = shstr_off + s.sh_name as usize;
			let section_name_end = section_name_begin + name.len();

			if section_name_end <= self.image.len() {
				let section_name = &self.image[section_name_begin..section_name_end];
				section_name == name.as_bytes()
			} else {
				false
			}
		})
	}

	/// Returns the symbol with name `name`.
	///
	/// If the symbol doesn't exist, the function returns `None`.
	pub fn get_symbol_by_name(&self, name: &str) -> Option<&ELF32Sym> {
		let strtab_section = self.get_section_by_name(".strtab")?; // TODO Use sh_link

		self.iter_sections()
			.flat_map(|s| {
				self.iter_symbols(s).filter(|sym| {
					let sym_name_begin = strtab_section.sh_offset as usize + sym.st_name as usize;
					let sym_name_end = sym_name_begin + name.len();

					if sym_name_end <= self.image.len() {
						let sym_name = &self.image[sym_name_begin..sym_name_end];
						sym_name == name.as_bytes()
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
			let begin_off = (strtab.sh_offset + sym.st_name) as usize;
			let ptr = &self.image[begin_off];
			let len = unsafe {
				// Safe because limited to the size of the section
				util::strnlen(ptr, (strtab.sh_size - sym.st_name) as _)
			};

			Some(&self.image[begin_off..(begin_off + len)])
		} else {
			None
		}
	}

	/// Returns the path to the ELF's interpreter.
	///
	/// If the ELF doesn't have an interpreter, the function returns `None`.
	pub fn get_interpreter_path(&self) -> Option<&[u8]> {
		self.iter_segments()
			.filter(|seg| seg.p_type == PT_INTERP)
			.map(|seg| {
				let begin = seg.p_offset as usize;
				let end = begin + seg.p_filesz as usize;
				// The slice won't exceed the size of the image since this is checked at parser
				// instanciation
				let path = &self.image[begin..end];

				// Exclude trailing `\0` if present
				if let Some(i) = path.iter().position(|c| *c == b'\0') {
					&path[..i]
				} else {
					path
				}
			})
			.next()
	}
}
