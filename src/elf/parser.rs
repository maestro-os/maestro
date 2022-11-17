//! This module implements the ELF parser.

use core::mem::size_of;
use crate::elf::relocation::ELF32Rel;
use crate::elf::relocation::ELF32Rela;
use crate::errno::Errno;
use crate::errno;
use super::*;
use super::iter::ELFIterator;

/// The ELF parser allows to parse an ELF image and retrieve informations on it.
/// It is especially useful to load a kernel module or a userspace program.
pub struct ELFParser<'a> {
	/// The ELF image.
	image: &'a [u8],
}

impl<'a> ELFParser<'a> {
	/// Returns the image's header.
	pub fn get_header(&self) -> &ELF32ELFHeader {
		// Safe because the image is already checked to be large enough on parser instanciation
		unsafe {
			util::reinterpret::<ELF32ELFHeader>(self.image)
		}.unwrap()
	}

	/// Returns the offset the content of the section containing section names.
	pub fn get_shstr_offset(&self) -> usize {
		let ehdr = self.get_header();
		let shoff = ehdr.e_shoff;
		let shentsize = ehdr.e_shentsize;

		// The offset of the section containing section names
		let shstr_off = (shoff + shentsize as u32 * ehdr.e_shstrndx as u32) as usize;

		// Safe because the image is already checked to be large enough on parser instanciation
		let shstr = unsafe {
			util::reinterpret::<ELF32SectionHeader>(&self.image[shstr_off..])
		}.unwrap();
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

		let phdr_end = ehdr.e_phoff as usize
			+ ehdr.e_phentsize as usize * ehdr.e_phnum as usize;
		if phdr_end > self.image.len() {
			return Err(errno!(EINVAL));
		}

		let shdr_end = ehdr.e_shoff as usize
			+ ehdr.e_shentsize as usize * ehdr.e_shnum as usize;
		if shdr_end > self.image.len() {
			return Err(errno!(EINVAL));
		}
		if ehdr.e_shstrndx >= ehdr.e_shnum {
			return Err(errno!(EINVAL));
		}

		for i in 0..ehdr.e_phnum {
			let off = (ehdr.e_phoff + ehdr.e_phentsize as u32 * i as u32) as usize;
			let phdr = util::reinterpret::<ELF32ProgramHeader>(&self.image[off..]).unwrap();

			phdr.is_valid(self.image.len())?;
		}

		for i in 0..ehdr.e_shnum {
			let off = (ehdr.e_shoff + ehdr.e_shentsize as u32 * i as u32) as usize;
			let shdr = util::reinterpret::<ELF32SectionHeader>(&self.image[off..]).unwrap();

			shdr.is_valid(self.image.len())?;
		}

		Ok(())
	}

	/// Creates a new instance for the given image.
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

	/// Iterates on every relocations that don't have an addend and calls the
	/// function `f` for each.
	/// The first argument of the closure is the header of the section
	/// containing the relocation and the second argument is the relocation.
	/// If the function returns `false`, the loop breaks.
	pub fn foreach_rel<F: FnMut(&ELF32SectionHeader, &ELF32Rel) -> bool>(&self, mut f: F) {
		self.foreach_sections(|_, section| {
			if section.sh_type != SHT_REL {
				return true;
			}

			let shoff = section.sh_offset;
			let entsize = section.sh_entsize;
			let num = section.sh_size / entsize;

			for i in 0..num {
				let off = (shoff + entsize as u32 * i as u32) as usize;
				let hdr = self.get_struct::<ELF32Rel>(off);

				if !f(section, hdr) {
					return false;
				}
			}

			true
		});
	}

	/// Iterates on every relocations that have an addend and calls the function
	/// `f` for each. The first argument of the closure is the header of the
	/// section containing the relocation and the second argument is the
	/// relocation. If the function returns `false`, the loop breaks.
	pub fn foreach_rela<F: FnMut(&ELF32SectionHeader, &ELF32Rela) -> bool>(&self, mut f: F) {
		self.foreach_sections(|_, section| {
			if section.sh_type != SHT_RELA {
				return true;
			}

			let shoff = section.sh_offset;
			let entsize = section.sh_entsize;
			let num = section.sh_size / entsize;

			for i in 0..num {
				let off = (shoff + entsize as u32 * i as u32) as usize;
				let hdr = self.get_struct::<ELF32Rela>(off);

				if !f(section, hdr) {
					return false;
				}
			}

			true
		});
	}

	/// Calls the given function `f` for each symbol in the image.
	/// The first argument of the function is the offset of the symbol in the
	/// image. The second argument is a reference to the symbol.
	/// If the function returns `false`, the loop breaks.
	pub fn foreach_symbol<F: FnMut(usize, &ELF32Sym) -> bool>(&self, mut f: F) {
		self.foreach_sections(|_, section| {
			if section.sh_type == SHT_SYMTAB {
				let begin = section.sh_offset;
				let mut i = 0;

				// TODO When checking the image, check the size of the section is a multiple of
				// the size of a symbol
				while i < section.sh_size {
					let off = begin as usize + i as usize;
					let sym = unsafe {
						// Safe because the slice is large enough
						&*(&self.image[off] as *const u8 as *const ELF32Sym)
					};

					if !f(off, sym) {
						return false;
					}

					i += section.sh_entsize;
				}
			}

			true
		});
	}

	/// Returns the section with name `name`. If the section doesn't exist, the
	/// function returns None.
	pub fn get_section_by_name(&self, name: &str) -> Option<&ELF32SectionHeader> {
		let shstr_off = self.get_shstr_offset();

		self.iter_sections()
			.filter(|s| {
				let section_name_begin = shstr_off + s.sh_name as usize;
				let section_name_end = section_name_begin + name.len();

				if section_name_end <= self.image.len() {
					let section_name = &self.image[section_name_begin..section_name_end];
					section_name == name.as_bytes()
				} else {
					false
				}
			})
			.next()
	}

	/// Returns the symbol with name `name`. If the symbol doesn't exist, the
	/// function returns None.
	pub fn get_symbol_by_name(&self, name: &str) -> Option<&ELF32Sym> {
		let strtab_section = self.get_section_by_name(".strtab")?; // TODO Use sh_link
		let mut r = None;

		self.foreach_symbol(|off, sym| {
			let sym_name = &self.image[(strtab_section.sh_offset + sym.st_name) as usize..];

			if &sym_name[..min(sym_name.len(), name.len())] == name.as_bytes() {
				r = Some(off);
				false
			} else {
				true
			}
		});

		Some(self.get_struct::<ELF32Sym>(r?))
	}

	/// Returns the name of the symbol `sym` using the string table section
	/// `strtab`. If the symbol name doesn't exist, the function returns None.
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
	/// If the ELF doesn't have an interpreter, the function returns None.
	pub fn get_interpreter_path(&self) -> Option<&[u8]> {
		self.iter_segments()
			.filter(|seg| seg.p_type == PT_INTERP)
			.map(|seg| {
				let begin = seg.p_offset as usize;
				let end = (seg.p_offset + seg.p_filesz) as usize;
				// The slice won't exceed the size of the image since this is checked at parser
				// instanciation
				let path = &self.image[begin..end];

				// Exclude trailing `\0` if present
				if let Some(i) = path.iter().position(|c| *c == b'\0') {
					path = &path[..i];
				}

				path
			})
			.next()
	}
}
