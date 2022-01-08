//! This module implements the ELF parser.

use crate::elf::relocation::ELF32Rel;
use crate::elf::relocation::ELF32Rela;
use crate::errno::Errno;
use crate::errno;
use super::*;

/// The ELF parser allows to parse an ELF image and retrieve informations on it.
/// It is especially useful to load a kernel module or a userspace program.
pub struct ELFParser<'a> {
	/// The ELF image.
	image: &'a [u8],
}

impl<'a> ELFParser<'a> {
	/// Returns the image's header.
	/// If the image is invalid, the behaviour is undefined.
	pub fn get_header(&self) -> &ELF32ELFHeader {
		unsafe { // Safe because the slice is large enough
			&*(&self.image[0] as *const u8 as *const ELF32ELFHeader)
		}
	}

	/// Returns the structure at offset `off`. The generic argument `T` tells which structure to
	/// return.
	/// If the image is invalid or if the offset is outside of the image, the behaviour is
	/// undefined.
	pub fn get_struct<T>(&self, off: usize) -> &T {
		debug_assert!(off < self.image.len());

		unsafe { // Safe because the slice is large enough
			&*(&self.image[off] as *const u8 as *const T)
		}
	}

	/// Returns the offset the content of the section containing section names.
	pub fn get_shstr_offset(&self) -> usize {
		let ehdr = self.get_header();
		let shoff = ehdr.e_shoff;
		let shentsize = ehdr.e_shentsize;

		// The offset of the section containing section names
		let shstr_off = (shoff + shentsize as u32 * ehdr.e_shstrndx as u32) as usize;
		// The header of the section containing section names
		let shstr = self.get_struct::<ELF32SectionHeader>(shstr_off);

		shstr.sh_offset as _
	}

	// TODO Support 64 bit
	/// Tells whether the ELF image is valid.
	fn check_image(&self) -> bool {
		let signature = [0x7f, b'E', b'L', b'F'];

		if self.image.len() < EI_NIDENT {
			return false;
		}
		if self.image[0..signature.len()] != signature {
			return false;
		}

		// TODO Check relative to current architecture
		if self.image[EI_CLASS] != ELFCLASS32 {
			return false;
		}

		// TODO Check relative to current architecture
		if self.image[EI_DATA] != ELFDATA2LSB {
			return false;
		}

		if self.image.len() < size_of::<ELF32ELFHeader>() {
			return false;
		}
		let ehdr = self.get_header();

		// TODO Check e_machine
		// TODO Check e_version

		if ehdr.e_ehsize != size_of::<ELF32ELFHeader>() as u16 {
			return false;
		}

		if ehdr.e_phoff + ehdr.e_phentsize as u32 * ehdr.e_phnum as u32 > self.image.len() as u32 {
			return false;
		}
		if ehdr.e_shoff + ehdr.e_shentsize as u32 * ehdr.e_shnum as u32 > self.image.len() as u32 {
			return false;
		}
		if ehdr.e_shstrndx >= ehdr.e_shnum {
			return false;
		}

		for i in 0..ehdr.e_phnum {
			let off = (ehdr.e_phoff + ehdr.e_phentsize as u32 * i as u32) as usize;
			let phdr = self.get_struct::<ELF32ProgramHeader>(off);

			if !phdr.is_valid(self.image.len()) {
				return false;
			}
		}

		for i in 0..ehdr.e_shnum {
			let off = (ehdr.e_shoff + ehdr.e_shentsize as u32 * i as u32) as usize;
			let shdr = self.get_struct::<ELF32SectionHeader>(off);

			if !shdr.is_valid(self.image.len()) {
				return false;
			}
		}

		true
	}

	/// Creates a new instance for the given image.
	/// The function checks if the image is valid. If not, the function retuns an error.
	pub fn new(image: &'a [u8]) -> Result<Self, Errno> {
		let p = Self {
			image,
		};

		if p.check_image() {
			Ok(p)
		} else {
			Err(errno::EINVAL)
		}
	}

	/// Returns a reference to the ELF image.
	pub fn get_image(&self) -> &[u8] {
		&self.image
	}

	/// Calls the given function `f` for each segments in the image.
	/// If the function returns `false`, the loop breaks.
	pub fn foreach_segments<F: FnMut(&ELF32ProgramHeader) -> bool>(&self, mut f: F) {
		let ehdr = self.get_header();
		let phoff = ehdr.e_phoff;
		let phnum = ehdr.e_phnum;
		let phentsize = ehdr.e_phentsize;

		for i in 0..phnum {
			let off = (phoff + phentsize as u32 * i as u32) as usize;
			let hdr = self.get_struct::<ELF32ProgramHeader>(off);

			if !f(hdr) {
				break;
			}
		}
	}

	/// Calls the given function `f` for each section in the image.
	/// The first argument of the function is the offset of the section header in the image.
	/// The second argument is a reference to the section header.
	/// If the function returns `false`, the loop breaks.
	pub fn foreach_sections<F: FnMut(usize, &ELF32SectionHeader) -> bool>(&self, mut f: F) {
		let ehdr = self.get_header();
		let shoff = ehdr.e_shoff;
		let shnum = ehdr.e_shnum;
		let shentsize = ehdr.e_shentsize;

		for i in 0..shnum {
			let off = (shoff + shentsize as u32 * i as u32) as usize;
			let hdr = self.get_struct::<ELF32SectionHeader>(off);

			if !f(off, hdr) {
				break;
			}
		}
	}

	/// Iterates on every relocations that don't have an addend and calls the function `f` for
	/// each.
	/// The first argument of the closure is the header of the section containing the relocation
	/// and the second argument is the relocation.
	/// If the function returns `false`, the loop breaks.
	pub fn foreach_rel<F: FnMut(&ELF32SectionHeader, &ELF32Rel) -> bool>(&self, mut f: F) {
		self.foreach_sections(| _, section | {
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

	/// Iterates on every relocations that have an addend and calls the function `f` for each.
	/// The first argument of the closure is the header of the section containing the relocation
	/// and the second argument is the relocation.
	/// If the function returns `false`, the loop breaks.
	pub fn foreach_rela<F: FnMut(&ELF32SectionHeader, &ELF32Rela) -> bool>(&self, mut f: F) {
		self.foreach_sections(| _, section | {
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
	/// The first argument of the function is the offset of the symbol in the image.
	/// The second argument is a reference to the symbol.
	/// If the function returns `false`, the loop breaks.
	pub fn foreach_symbol<F: FnMut(usize, &ELF32Sym) -> bool>(&self, mut f: F) {
		self.foreach_sections(| _, section | {
			if section.sh_type == SHT_SYMTAB {
				let begin = section.sh_offset;
				let mut i = 0;

				// TODO When checking the image, check the size of the section is a multiple of the
				// size of a symbol
				while i < section.sh_size {
					let off = begin as usize + i as usize;
					let sym = unsafe { // Safe because the slice is large enough
						&*(&self.image[off] as *const u8 as *const ELF32Sym)
					};

					if !f(off, sym) {
						return false;
					}

					i += size_of::<ELF32Sym>() as u32;
				}
			}

			true
		});
	}

	/// Returns the section with index `section_index`. If the section doesn't exist, the function
	/// return None.
	pub fn get_section_by_index(&self, section_index: u32) -> Option<&ELF32SectionHeader> {
		let ehdr = self.get_header();
		if section_index >= ehdr.e_shnum as u32 {
			return None;
		}

		let section_off = (ehdr.e_shoff + ehdr.e_shentsize as u32 * section_index as u32) as usize;
		Some(self.get_struct::<ELF32SectionHeader>(section_off))
	}

	/// Returns the section with name `name`. If the section doesn't exist, the function returns
	/// None.
	pub fn get_section_by_name(&self, name: &str) -> Option<&ELF32SectionHeader> {
		let shstr_off = self.get_shstr_offset();
		let mut r = None;

		self.foreach_sections(| off, section | {
			let section_name = &self.image[(shstr_off + section.sh_name as usize)..];

			if &section_name[..min(section_name.len(), name.len())] == name.as_bytes() {
				r = Some(off);
				false
			} else {
				true
			}
		});

		Some(self.get_struct::<ELF32SectionHeader>(r?))
	}

	/// Returns the symbol with the given section and symbol index. If the symbol doesn't exist,
	/// the function returns None.
	/// `section` is the symbol's section.
	/// `symbol_index` is the symbol index.
	pub fn get_symbol_by_index(&self, section: &ELF32SectionHeader, symbol_index: u32)
		-> Option<&ELF32Sym> {
		if section.sh_type != SHT_SYMTAB && section.sh_type != SHT_DYNSYM {
			return None;
		}
		if symbol_index >= section.sh_size / section.sh_entsize {
			return None;
		}

		let sym_off = (section.sh_offset + section.sh_entsize * symbol_index as u32) as usize;
		Some(self.get_struct::<ELF32Sym>(sym_off))
	}

	/// Returns the symbol with name `name`. If the symbol doesn't exist, the function returns
	/// None.
	pub fn get_symbol_by_name(&self, name: &str) -> Option<&ELF32Sym> {
		let strtab_section = self.get_section_by_name(".strtab")?; // TODO Use sh_link
		let mut r = None;

		self.foreach_symbol(| off, sym | {
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

	/// Returns the name of the symbol `sym` using the string table section `strtab`. If the symbol
	/// name doesn't exist, the function returns None.
	pub fn get_symbol_name(&self, strtab: &ELF32SectionHeader, sym: &ELF32Sym) -> Option<&[u8]> {
		if sym.st_name != 0 {
			let begin_off = (strtab.sh_offset + sym.st_name) as usize;
			let ptr = &self.image[begin_off];
			let len = unsafe { // Safe because limited to the size of the section
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
		let mut path: Option<&[u8]> = None;

		self.foreach_segments(| segment | {
			if segment.p_type == PT_INTERP {
				let begin = segment.p_offset as usize;
				let end = (segment.p_offset + segment.p_filesz) as usize;
				// TODO Ensure the slice doesn't exceed the size of the image
				path = Some(&self.image[begin..end]);

				false
			} else {
				true
			}
		});

		path
	}
}
